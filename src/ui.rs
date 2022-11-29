use crate::app::{App, InputMode};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, Spans},
    widgets::canvas::{Canvas, Line, Map, MapResolution, Rectangle},
    widgets::{
        Axis, BarChart, Block, Borders, Cell, Chart, Dataset, Gauge, LineGauge, List, ListItem,
        Paragraph, Row, Sparkline, Table, Tabs, Wrap,
    },
    Frame,
};
use unicode_width::UnicodeWidthStr;

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(f.size());
    let titles = app
        .groups
        .items
        .iter()
        .map(|t| {
            Spans::from(Span::styled(
                &t.display_name,
                Style::default().fg(Color::Green),
            ))
        })
        .collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("groups"))
        .highlight_style(Style::default().fg(Color::Yellow))
        .select(app.groups.state.selected().unwrap_or(0));
    f.render_widget(tabs, chunks[0]);
    // match app.tabs.index {
    draw_choosen_tab(f, app, chunks[1]);
    // 1 => draw_second_tab(f, app, chunks[1]),
    // 2 => draw_third_tab(f, app, chunks[1]),
    // _ => {}
    // };
}

fn draw_choosen_tab<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let chunks = Layout::default()
        .constraints([Constraint::Min(5), Constraint::Length(3)].as_ref())
        .split(area);
    draw_dialog(f, app, chunks[0]);
    draw_user_input(f, app, chunks[1]);
}

fn draw_user_input<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let input = Paragraph::new(app.input.as_ref())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(Block::default().borders(Borders::TOP).title("Input"));
    f.render_widget(input, area);
    match app.input_mode {
        InputMode::Normal =>
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            {}

        InputMode::Editing => {
            // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
            f.set_cursor(
                // Put cursor past the end of the input text
                area.x + app.input.width() as u16,
                // Move one line down, from the border to the input line
                area.y + 1,
            )
        }
    }
}

fn draw_dialog<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let constraints = vec![Constraint::Percentage(80), Constraint::Percentage(20)];
    let chunks = Layout::default()
        .constraints(constraints)
        .direction(Direction::Horizontal)
        .split(area);
    let speaker_name_style = Style::default().fg(Color::Blue);
    // Draw tasks
    let tasks: Vec<ListItem> = app
        .tasks
        .items
        .iter()
        .map(|m| {
            ListItem::new(vec![Spans::from(vec![
                Span::styled(&m.speaker, speaker_name_style),
                Span::raw(" >>>> "),
                Span::raw(&m.message),
            ])])
        })
        .collect();
    let tasks = List::new(tasks)
        .block(Block::default().borders(Borders::TOP).title("dialog"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
    f.render_stateful_widget(tasks, chunks[0], &mut app.tasks.state);
}

// fn draw_text<B>(f: &mut Frame<B>, area: Rect)
// where
//     B: Backend,
// {
//     let text = vec![
//         Spans::from("This is a paragraph with several lines. You can change style your text the way you want"),
//         Spans::from(""),
//         Spans::from(vec![
//             Span::from("For example: "),
//             Span::styled("under", Style::default().fg(Color::Red)),
//             Span::raw(" "),
//             Span::styled("the", Style::default().fg(Color::Green)),
//             Span::raw(" "),
//             Span::styled("rainbow", Style::default().fg(Color::Blue)),
//             Span::raw("."),
//         ]),
//         Spans::from(vec![
//             Span::raw("Oh and if you didn't "),
//             Span::styled("notice", Style::default().add_modifier(Modifier::ITALIC)),
//             Span::raw(" you can "),
//             Span::styled("automatically", Style::default().add_modifier(Modifier::BOLD)),
//             Span::raw(" "),
//             Span::styled("wrap", Style::default().add_modifier(Modifier::REVERSED)),
//             Span::raw(" your "),
//             Span::styled("text", Style::default().add_modifier(Modifier::UNDERLINED)),
//             Span::raw(".")
//         ]),
//         Spans::from(
//             "One more thing is that it should display unicode characters: 10€"
//         ),
//     ];
//     let block = Block::default().borders(Borders::ALL).title(Span::styled(
//         "Footer",
//         Style::default()
//             .fg(Color::Magenta)
//             .add_modifier(Modifier::BOLD),
//     ));
//     let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
//     f.render_widget(paragraph, area);
// }

// fn draw_second_tab<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
// where
//     B: Backend,
// {
//     let chunks = Layout::default()
//         .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
//         .direction(Direction::Horizontal)
//         .split(area);
//     let up_style = Style::default().fg(Color::Green);
//     let failure_style = Style::default()
//         .fg(Color::Red)
//         .add_modifier(Modifier::RAPID_BLINK | Modifier::CROSSED_OUT);
//     let rows = app.servers.iter().map(|s| {
//         let style = if s.status == "Up" {
//             up_style
//         } else {
//             failure_style
//         };
//         Row::new(vec![s.name, s.location, s.status]).style(style)
//     });
//     let table = Table::new(rows)
//         .header(
//             Row::new(vec!["Server", "Location", "Status"])
//                 .style(Style::default().fg(Color::Yellow))
//                 .bottom_margin(1),
//         )
//         .block(Block::default().title("Servers").borders(Borders::ALL))
//         .widths(&[
//             Constraint::Length(15),
//             Constraint::Length(15),
//             Constraint::Length(10),
//         ]);
//     f.render_widget(table, chunks[0]);

//     let map = Canvas::default()
//         .block(Block::default().title("World").borders(Borders::ALL))
//         .paint(|ctx| {
//             ctx.draw(&Map {
//                 color: Color::White,
//                 resolution: MapResolution::High,
//             });
//             ctx.layer();
//             ctx.draw(&Rectangle {
//                 x: 0.0,
//                 y: 30.0,
//                 width: 10.0,
//                 height: 10.0,
//                 color: Color::Yellow,
//             });
//             for (i, s1) in app.servers.iter().enumerate() {
//                 for s2 in &app.servers[i + 1..] {
//                     ctx.draw(&Line {
//                         x1: s1.coords.1,
//                         y1: s1.coords.0,
//                         y2: s2.coords.0,
//                         x2: s2.coords.1,
//                         color: Color::Yellow,
//                     });
//                 }
//             }
//             for server in &app.servers {
//                 let color = if server.status == "Up" {
//                     Color::Green
//                 } else {
//                     Color::Red
//                 };
//                 ctx.print(
//                     server.coords.1,
//                     server.coords.0,
//                     Span::styled("X", Style::default().fg(color)),
//                 );
//             }
//         })
//         .marker(if app.enhanced_graphics {
//             symbols::Marker::Braille
//         } else {
//             symbols::Marker::Dot
//         })
//         .x_bounds([-180.0, 180.0])
//         .y_bounds([-90.0, 90.0]);
//     f.render_widget(map, chunks[1]);
// }

// fn draw_third_tab<B>(f: &mut Frame<B>, _app: &mut App, area: Rect)
// where
//     B: Backend,
// {
//     let chunks = Layout::default()
//         .direction(Direction::Horizontal)
//         .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
//         .split(area);
//     let colors = [
//         Color::Reset,
//         Color::Black,
//         Color::Red,
//         Color::Green,
//         Color::Yellow,
//         Color::Blue,
//         Color::Magenta,
//         Color::Cyan,
//         Color::Gray,
//         Color::DarkGray,
//         Color::LightRed,
//         Color::LightGreen,
//         Color::LightYellow,
//         Color::LightBlue,
//         Color::LightMagenta,
//         Color::LightCyan,
//         Color::White,
//     ];
//     let items: Vec<Row> = colors
//         .iter()
//         .map(|c| {
//             let cells = vec![
//                 Cell::from(Span::raw(format!("{:?}: ", c))),
//                 Cell::from(Span::styled("Foreground", Style::default().fg(*c))),
//                 Cell::from(Span::styled("Background", Style::default().bg(*c))),
//             ];
//             Row::new(cells)
//         })
//         .collect();
//     let table = Table::new(items)
//         .block(Block::default().title("Colors").borders(Borders::ALL))
//         .widths(&[
//             Constraint::Ratio(1, 3),
//             Constraint::Ratio(1, 3),
//             Constraint::Ratio(1, 3),
//         ]);
//     f.render_widget(table, chunks[0]);
// }
