//! TUI overview with graphs for the tomato42 plant simulator.
//!
//! This application provides a text-based user interface with time-series graphs
//! showing the internal state of the tomato plant.

use std::io;
use std::time::{Duration, Instant};
use std::error::Error;
use ringbuffer::RingBufferWrite;
use ringbuffer::RingBufferExt;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, Spans},
    widgets::{Axis, Block, Borders, Chart, Dataset, Paragraph, Wrap},
    Frame, Terminal,
};
use ringbuffer::{AllocRingBuffer, RingBuffer};

use tomato42_core::{Action, Event as TomatoEvent, Stage, TomatoState, step};

const BUFFER_SIZE: usize = 100;

/// Stores historical data for graphing
struct HistoricalData {
    time_points: AllocRingBuffer<f64>,
    soil_moisture: AllocRingBuffer<f64>,
    stress: AllocRingBuffer<f64>,
    health: AllocRingBuffer<f64>,
    biomass: AllocRingBuffer<f64>,
}

impl HistoricalData {
    fn new(capacity: usize) -> Self {
        Self {
            time_points: AllocRingBuffer::new(),
            soil_moisture: AllocRingBuffer::new(),
            stress: AllocRingBuffer::new(),
            health: AllocRingBuffer::new(),
            biomass: AllocRingBuffer::new(),
        }
    }

    fn add_data_point(&mut self, state: &TomatoState) {
        let time = state.time.as_secs_f64();
        self.time_points.push(time);
        self.soil_moisture.push(state.soil_moisture as f64);
        self.stress.push(state.stress as f64);
        self.health.push(state.health as f64);
        self.biomass.push(state.biomass as f64);
    }

    fn get_data_points(&self, buffer: &AllocRingBuffer<f64>) -> Vec<(f64, f64)> {
        self.time_points
            .iter()
            .zip(buffer.iter())
            .map(|(&x, &y)| (x, y))
            .collect()
    }
}

/// Application state
struct App {
    state: TomatoState,
    historical_data: HistoricalData,
    last_events: Vec<TomatoEvent>,
    last_update: Instant,
    auto_step: bool,
    auto_step_interval: Duration,
    selected_action: Action,
    water_amount: f32,
    light_level: f32,
    temperature: f32,
}

impl App {
    fn new() -> Self {
        let state = TomatoState::new();
        let mut historical_data = HistoricalData::new(BUFFER_SIZE);
        historical_data.add_data_point(&state);

        Self {
            state,
            historical_data,
            last_events: Vec::new(),
            last_update: Instant::now(),
            auto_step: false,
            auto_step_interval: Duration::from_millis(500),
            selected_action: Action::DoNothing,
            water_amount: 0.5,
            light_level: 0.5,
            temperature: 20.0,
        }
    }

    fn step(&mut self) {
        let result = step(self.state.clone(), self.selected_action, Duration::from_secs(1));
        self.state = result.state.clone();
        self.last_events = result.events;
        self.historical_data.add_data_point(&self.state);
        self.selected_action = Action::DoNothing; // Reset to DoNothing after each step
    }

    fn update(&mut self) -> bool {
        if self.auto_step && self.last_update.elapsed() >= self.auto_step_interval {
            self.step();
            self.last_update = Instant::now();
            return true;
        }
        false
    }

    fn toggle_auto_step(&mut self) {
        self.auto_step = !self.auto_step;
        self.last_update = Instant::now();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let app = App::new();
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        // Check for auto-step update
        if app.update() {
            continue;
        }

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char(' ') => app.step(),
                    KeyCode::Char('a') => app.toggle_auto_step(),
                    KeyCode::Char('w') => {
                        app.selected_action = Action::Water { amount: app.water_amount };
                        app.step();
                    }
                    KeyCode::Char('l') => {
                        app.selected_action = Action::SetLight { level: app.light_level };
                        app.step();
                    }
                    KeyCode::Char('t') => {
                        app.selected_action = Action::SetTemp { celsius: app.temperature };
                        app.step();
                    }
                    KeyCode::Up => app.water_amount = (app.water_amount + 0.1).min(1.0),
                    KeyCode::Down => app.water_amount = (app.water_amount - 0.1).max(0.0),
                    KeyCode::Right => app.light_level = (app.light_level + 0.1).min(1.0),
                    KeyCode::Left => app.light_level = (app.light_level - 0.1).max(0.0),
                    KeyCode::Char('+') => app.temperature += 1.0,
                    KeyCode::Char('-') => app.temperature -= 1.0,
                    _ => {}
                }
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    // Create layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(5),  // Status
            Constraint::Min(10),    // Charts
            Constraint::Length(5),  // Controls
            Constraint::Length(5),  // Events
        ].as_ref())
        .split(f.size());

    // Title
    let title = Paragraph::new("Tomato42 - Deterministic Tomato Plant Simulator")
        .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Status
    render_status(f, app, chunks[1]);

    // Charts area
    let charts_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ].as_ref())
        .split(chunks[2]);

    let top_charts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ].as_ref())
        .split(charts_layout[0]);

    let bottom_charts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ].as_ref())
        .split(charts_layout[1]);

    // Render charts
    render_chart(f, app, "Soil Moisture", &app.historical_data.soil_moisture, Color::Blue, top_charts[0]);
    render_chart(f, app, "Stress", &app.historical_data.stress, Color::Red, top_charts[1]);
    render_chart(f, app, "Health", &app.historical_data.health, Color::Green, bottom_charts[0]);
    render_chart(f, app, "Biomass", &app.historical_data.biomass, Color::Yellow, bottom_charts[1]);

    // Controls
    render_controls(f, app, chunks[3]);

    // Events
    render_events(f, app, chunks[4]);
}

fn render_status<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let status_text = vec![
        Spans::from(vec![
            Span::raw("Stage: "),
            Span::styled(
                format!("{:?}", app.state.stage),
                Style::default().fg(get_stage_color(&app.state.stage)),
            ),
            Span::raw(" | Time: "),
            Span::raw(format!("{}s", app.state.time.as_secs())),
            Span::raw(" | Auto: "),
            Span::styled(
                if app.auto_step { "ON" } else { "OFF" },
                Style::default().fg(if app.auto_step { Color::Green } else { Color::Red }),
            ),
        ]),
        Spans::from(vec![
            Span::raw("Temperature: "),
            Span::raw(format!("{:.1}°C", app.state.temperature)),
            Span::raw(" | Light: "),
            Span::raw(format!("{:.2}", app.state.light_level)),
        ]),
    ];

    let status = Paragraph::new(status_text)
        .block(Block::default().title("Status").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    f.render_widget(status, area);
}

fn render_chart<B: Backend>(
    f: &mut Frame<B>,
    app: &App,
    title: &str,
    data_buffer: &AllocRingBuffer<f64>,
    color: Color,
    area: Rect,
) {
    let data = app.historical_data.get_data_points(data_buffer);
    
    // Find min/max values for y-axis
    let min_y = data.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
    let max_y = data.iter().map(|(_, y)| *y).fold(f64::NEG_INFINITY, f64::max);
    let y_bounds = [min_y.max(0.0), max_y.max(1.0)];
    
    // Find min/max values for x-axis
    let min_x = data.iter().map(|(x, _)| *x).fold(f64::INFINITY, f64::min);
    let max_x = data.iter().map(|(x, _)| *x).fold(f64::NEG_INFINITY, f64::max);
    let x_bounds = [min_x.max(0.0), max_x.max(1.0)];

    let datasets = vec![Dataset::default()
        .name(title)
        .marker(symbols::Marker::Braille)
        .style(Style::default().fg(color))
        .data(&data)];

    let chart = Chart::new(datasets)
        .block(Block::default().title(title).borders(Borders::ALL))
        .x_axis(
            Axis::default()
                .title("Time (s)")
                .style(Style::default().fg(Color::Gray))
                .bounds(x_bounds)
                .labels(vec![
                    Span::styled(format!("{:.0}", x_bounds[0]), Style::default().fg(Color::Gray)),
                    Span::styled(format!("{:.0}", x_bounds[1]), Style::default().fg(Color::Gray)),
                ]),
        )
        .y_axis(
            Axis::default()
                .title("Value")
                .style(Style::default().fg(Color::Gray))
                .bounds(y_bounds)
                .labels(vec![
                    Span::styled(format!("{:.1}", y_bounds[0]), Style::default().fg(Color::Gray)),
                    Span::styled(format!("{:.1}", y_bounds[1]), Style::default().fg(Color::Gray)),
                ]),
        );
    f.render_widget(chart, area);
}

fn render_controls<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let controls_text = vec![
        Spans::from(vec![
            Span::styled("Controls: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("q: Quit | Space: Step | a: Toggle Auto"),
        ]),
        Spans::from(vec![
            Span::raw("w: Water ("),
            Span::styled(format!("{:.1}", app.water_amount), Style::default().fg(Color::Blue)),
            Span::raw(") | l: Light ("),
            Span::styled(format!("{:.1}", app.light_level), Style::default().fg(Color::Yellow)),
            Span::raw(") | t: Temp ("),
            Span::styled(format!("{:.1}°C", app.temperature), Style::default().fg(Color::Red)),
            Span::raw(")"),
        ]),
        Spans::from(vec![
            Span::raw("↑/↓: Water amount | ←/→: Light level | +/-: Temperature"),
        ]),
    ];

    let controls = Paragraph::new(controls_text)
        .block(Block::default().title("Controls").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    f.render_widget(controls, area);
}

fn render_events<B: Backend>(f: &mut Frame<B>, app: &App, area: Rect) {
    let mut event_text = Vec::new();
    
    if app.last_events.is_empty() {
        event_text.push(Spans::from(Span::raw("No recent events")));
    } else {
        for event in &app.last_events {
            match event {
                TomatoEvent::StageChange { from, to } => {
                    event_text.push(Spans::from(vec![
                        Span::raw("Stage changed from "),
                        Span::styled(format!("{:?}", from), Style::default().fg(get_stage_color(from))),
                        Span::raw(" to "),
                        Span::styled(format!("{:?}", to), Style::default().fg(get_stage_color(to))),
                    ]));
                }
                TomatoEvent::WiltRisk => {
                    event_text.push(Spans::from(vec![
                        Span::styled("WARNING: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::raw("Plant is at risk of wilting due to high stress!"),
                    ]));
                }
                TomatoEvent::Death => {
                    event_text.push(Spans::from(vec![
                        Span::styled("ALERT: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                        Span::raw("Plant has died!"),
                    ]));
                }
            }
        }
    }

    let events = Paragraph::new(event_text)
        .block(Block::default().title("Events").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    f.render_widget(events, area);
}

fn get_stage_color(stage: &Stage) -> Color {
    match stage {
        Stage::Seed => Color::White,
        Stage::Seedling => Color::LightGreen,
        Stage::Vegetative => Color::Green,
        Stage::Flowering => Color::Magenta,
        Stage::Fruiting => Color::Red,
        Stage::Dead => Color::DarkGray,
    }
}