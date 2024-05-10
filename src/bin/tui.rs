use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    execute,
    style::Color,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    prelude::Style,
    widgets::{Block, Borders, List, ListItem, ListState},
    Terminal,
};

use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use req::*;

enum Event<I> {
    Input(I),
    Tick,
}

struct App {
    project: Project,
    topics_list_state: ListState,
    requirements_list_state: ListState,
}

impl App {
    fn new(project: Project) -> App {
        let mut topics_list_state = ListState::default();
        topics_list_state.select(Some(0));

        let mut requirements_list_state = ListState::default();
        requirements_list_state.select(Some(0));

        App {
            project,
            topics_list_state,
            requirements_list_state,
        }
    }

    fn draw(&mut self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Percentage(10), // Project Title
                    Constraint::Percentage(45), // Topics List
                    Constraint::Percentage(45), // Requirements List
                ]
                .as_ref(),
            )
            .split(f.size());

        let project_title = Block::default()
            .title(self.project.name.clone())
            .borders(Borders::ALL);
        f.render_widget(project_title, chunks[0]);

        let topics: Vec<ListItem> = self
            .project
            .topics
            .iter()
            .map(|(name, _)| ListItem::new(name.clone()))
            .collect();
        let topics_list = List::new(topics)
            .block(Block::default().borders(Borders::ALL).title("Topics"))
            .highlight_style(Style::default().bg(Color::Blue.into()));
        f.render_stateful_widget(topics_list, chunks[1], &mut self.topics_list_state);

        let requirements: Vec<ListItem> = self
            .project
            .topics
            .first()
            .unwrap()
            .1
            .requirements
            .iter()
            .map(|(name, _)| ListItem::new(name.clone()))
            .collect();
        let requirements_list = List::new(requirements)
            .block(Block::default().borders(Borders::ALL).title("Requirements"))
            .highlight_style(Style::default().bg(Color::Yellow.into()));
        f.render_stateful_widget(
            requirements_list,
            chunks[2],
            &mut self.requirements_list_state,
        );
    }

    fn next_topic(&mut self) {
        let n = self.project.topics.len();
        if let Some(i) = self.topics_list_state.selected() {
            if i >= n - 1 {
                self.topics_list_state.select(Some(0));
            } else {
                self.topics_list_state.select(Some(i + 1));
            }
        }
    }

    fn previous_topic(&mut self) {
        let n = self.project.topics.len();
        if let Some(i) = self.topics_list_state.selected() {
            if i == 0 {
                self.topics_list_state.select(Some(n - 1));
            } else {
                self.topics_list_state.select(Some(i - 1));
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let project = demo_project();

    let mut app = App::new(project);

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("read works") {
                    tx.send(Event::Input(key)).expect("send works");
                }
            }
            if last_tick.elapsed() >= tick_rate {
                tx.send(Event::Tick).expect("tick works");
                last_tick = Instant::now();
            }
        }
    });

    loop {
        terminal.draw(|f| app.draw(f))?;

        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    disable_raw_mode()?;
                    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                    terminal.show_cursor()?;
                    break;
                }
                KeyCode::Down => {
                    app.next_topic();
                }
                KeyCode::Up => {
                    app.previous_topic();
                }
                _ => {}
            },
            Event::Tick => {}
        }
    }

    Ok(())
}
