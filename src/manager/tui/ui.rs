use std::{collections::{HashMap, VecDeque}, sync::{Arc, Mutex}};

use tui::{layout::{Alignment, Constraint, Direction, Layout}, style::{Color, Modifier, Style}, symbols, widgets::{Axis, Block, Borders, Chart, Dataset, List, ListItem, Paragraph}};

use crate::centered_paragraph;

pub fn draw_ui<B: tui::backend::Backend>(
    f: &mut tui::Frame<B>,
    messages: &Arc<Mutex<HashMap<String, (String, Color)>>>,
    flash_state: &Arc<Mutex<bool>>,
    aggregator_data: &Arc<Mutex<String>>,
    git_data: &Arc<Mutex<String>>,
    aggregator_status: &Arc<Mutex<String>>,
    _cpu_usage: &Arc<Mutex<f64>>,
    _ram_usage: &Arc<Mutex<f64>>,
    system_stats: &Arc<Mutex<HashMap<String, String>>>,
    cpu_history: &Arc<Mutex<VecDeque<(f64, f64)>>>,
    ram_history: &Arc<Mutex<VecDeque<(f64, f64)>>>,
) {
    // Layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Percentage(35),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(20),
                Constraint::Percentage(25),
            ]
            .as_ref(),
        )
        .split(f.size());

    let upper_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ]
            .as_ref(),
        )
        .split(chunks[0]);

    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[3]);

    // let lower_chunks = Layout::default()
    //     .direction(Direction::Horizontal)
    //     .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
    //     .split(chunks[4]);

    // System stats block
    let stats = system_stats.lock().unwrap().clone();
    let stats_str = stats
        .into_iter()
        .map(|(k, v)| format!("{}: {}", k, v))
        .collect::<Vec<_>>()
        .join("\n");
    let block = Block::default()
        .title("System Stats")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(block, upper_chunks[0]);
    f.render_widget(centered_paragraph(stats_str), upper_chunks[0]);

    // Aggregator status block
    let block = Block::default()
        .title("Aggregator Stats")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Magenta));
    f.render_widget(block.clone(), upper_chunks[1]);
    let aggregator_str = aggregator_data.lock().unwrap().clone();
    f.render_widget(centered_paragraph(aggregator_str), upper_chunks[1]);

    // GitHub status block
    let style = Style::default().fg(Color::Yellow);
    let block = Block::default()
        .title("GitHub Stats")
        .borders(Borders::ALL)
        .style(style);
    f.render_widget(block.clone(), upper_chunks[2]);
    let git_str = git_data.lock().unwrap().clone();
    f.render_widget(centered_paragraph(git_str), upper_chunks[2]);


    // Main block with messages
    let flash = *flash_state.lock().unwrap();
    let items: Vec<ListItem> = {
        let msgs = messages.lock().unwrap();
        msgs.iter()
            .map(|(_key, (msg, color))| {
                if flash && *color == Color::Gray {
                    ListItem::new(msg.clone()).style(Style::default().fg(*color).add_modifier(Modifier::BOLD))
                } else {
                    ListItem::new(msg.clone()).style(Style::default().fg(*color))
                }
            })
            .collect()
    };
    let list = List::new(items).block(Block::default().title("Main").borders(Borders::ALL));
    f.render_widget(list, chunks[1]);

    // Aggregator status block
    let status = aggregator_status.lock().unwrap().clone();
    let status_color = if status == "OK" {
        Color::Green
    } else {
        Color::Red
    };
    let status_block = Block::default().title("Aggregator Status").borders(Borders::ALL);
    f.render_widget(status_block, chunks[2]);
    f.render_widget(
        Paragraph::new(status.clone())
            .alignment(Alignment::Center)
            .style(Style::default().fg(status_color)),
        chunks[2],
    );

    // CPU and RAM usage charts
    let cpu_history: VecDeque<(f64, f64)> = cpu_history.lock().unwrap().clone();
    let ram_history: VecDeque<(f64, f64)> = ram_history.lock().unwrap().clone();
    let cpu_history_iter: Vec<(f64, f64)> = cpu_history.iter().cloned().collect::<Vec<_>>();
    let ram_history_iter: Vec<(f64, f64)> = ram_history.iter().cloned().collect::<Vec<_>>();


    let cpu_dataset = vec![
        Dataset::default()
            .name("CPU Usage")
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(Color::Red))
            .data(&cpu_history_iter),
    ];

    let ram_dataset = vec![
        Dataset::default()
            .name("RAM Usage")
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(Color::Blue))
            .data(&ram_history_iter),
    ];

    let cpu_chart = Chart::new(cpu_dataset)
        .block(Block::default().title("CPU Usage Over Time").borders(Borders::ALL))
        .x_axis(Axis::default().title("Time").bounds([0.0, 100.0]))
        .y_axis(Axis::default().title("Usage %").bounds([0.0, 100.0]));

    let ram_chart = Chart::new(ram_dataset)
        .block(Block::default().title("RAM Usage Over Time").borders(Borders::ALL))
        .x_axis(Axis::default().title("Time").bounds([0.0, 100.0]))
        .y_axis(Axis::default().title("Usage GB").bounds([0.0, 100.0]));

    f.render_widget(cpu_chart, middle_chunks[0]);
    f.render_widget(ram_chart, middle_chunks[1]);

    // Helper text block
    let helper_text = "Key actions:\n\
                       q: Quit\n\
                       a: Query Aggregator Status\n\
                       g: Query GitHub Repo Status\n\
                       u: Update GitHub Repo";
    let helper_block = Block::default().title("Helper").borders(Borders::ALL);
    f.render_widget(helper_block, chunks[4]);
    f.render_widget(Paragraph::new(helper_text).alignment(Alignment::Center), chunks[4]);
}