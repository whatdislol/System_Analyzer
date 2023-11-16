use std::{io::{Stdout, Write}, fs::File, collections::VecDeque};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, Paragraph, ListItem, List, ListState, ScrollbarState, ScrollbarOrientation, Scrollbar},
    text::{ Line, Span},
    Frame,
    prelude::{Rect, Margin}, style::{Style, Color}, symbols::scrollbar,
};
use sysinfo::{System, SystemExt, CpuExt, NetworkExt, DiskExt, ProcessExt};
use chrono::{Local, Timelike};

#[derive(Default)]
pub struct App {
    pub vertical_scroll_state: ScrollbarState,
    pub horizontal_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
    pub horizontal_scroll: usize,
}

pub fn get_cpu(sys: &System) -> Vec<f32> {
    let mut cpu_list = vec![];
    for cpu in sys.cpus() {
        cpu_list.push(cpu.cpu_usage());
    }
    cpu_list
}

pub fn get_avg_cpu(cpu_list: &[f32]) -> f32 {
    let mut all_percentage: f32 = 0.;
    for cpu in cpu_list {
        all_percentage += cpu;
    }
    all_percentage / cpu_list.len() as f32
}

pub fn display_cpu(cpu_list: Vec<f32>, avg_cpu: f32, f: &mut Frame<'_, CrosstermBackend<Stdout>>, chunk: Rect){
    let mut cpu_list_report = vec![];
    for (i, cpu) in cpu_list.iter().enumerate() {
        let cpu_stat = format!("CPU {}: {:.2}%", i, cpu);
        cpu_list_report.push(Line::from(vec![Span::raw(cpu_stat)]));
    }
    let cpu_average = format!(
        "Average CPU: {:.2}%",
        avg_cpu
    );
    cpu_list_report.insert(0, Line::from(vec![Span::raw(cpu_average)]));
    cpu_list_report.insert(1, Line::from(vec![Span::raw("")]));

    let cpu_usage_paragraph = Paragraph::new(cpu_list_report)
        .block(Block::default().borders(Borders::ALL).title("CPU Usage").borders(Borders::ALL))
        .alignment(ratatui::layout::Alignment::Left);
    f.render_widget(cpu_usage_paragraph, chunk);
}

pub fn display_battery(f: &mut Frame<'_, CrosstermBackend<Stdout>>, chunk: Rect) -> Result<(), battery::Error> {
    let manager = battery::Manager::new()?;
    let mut battery_list = vec![];
    let mut battery_stat: String;

    for (idx, maybe_battery) in manager.batteries()?.enumerate() {
        match maybe_battery {
            Ok(battery) => {
                battery_stat = format!(
                    "Battery #{}\nVendor: {}\nModel: {}\nState: {}\nPercentage: {:.02?}%\n ",
                    idx, 
                    battery.vendor().unwrap_or("Unknown"),
                    battery.model().unwrap_or("Unknown"),
                    battery.state(),
                    battery.state_of_charge() * 100.
                );
            }
            Err(_) => {
                battery_stat = format!("Battery #{}\nState: No battery found.\n ", idx);
            }
        }
        for line in battery_stat.lines() {
            battery_list.push(Line::from(vec![Span::raw(line.to_string())]));
        }
    }
    let processes_paragraph = Paragraph::new(battery_list)
        .block((Block::default().borders(Borders::ALL)).title("Battery").borders(Borders::ALL))
        .alignment(ratatui::layout::Alignment::Left);
    f.render_widget(processes_paragraph, chunk);

    Ok(())
}

pub fn display_network(sys: &System, f: &mut Frame<'_, CrosstermBackend<Stdout>>, chunk: Rect){
    let mut network_list = vec![];
    let networks = sys.networks();
    for (interface_name, data) in networks {
        let network_stat = format!(
            "[{}]\nin: {:.2}KB, out: {:.2}KB\n ",
            interface_name,
            data.received() as f64 / 1e3,
            data.transmitted() as f64 / 1e3,
        );
        for line in network_stat.lines() {
            network_list.push(Line::from(vec![Span::raw(line.to_string())]));
        }
    }
    let network_paragraph = Paragraph::new(network_list)
        .block(Block::default().borders(Borders::ALL).title("Network").borders(Borders::ALL))
        .alignment(ratatui::layout::Alignment::Left);
    f.render_widget(network_paragraph, chunk);
}

pub fn display_memory(sys: &System, f: &mut Frame<'_, CrosstermBackend<Stdout>>, chunk: Rect){
    let mut memory_list_report = vec![];
    let memory_stat = format!(
        "Free Memory: {:.2?} GB\nUsed Memory: {:.2?} GB\nAvailable Memory: {:.2?} GB\nTotal Memory: {:.2?} GB",
        sys.free_memory() as f64 / 1e9,
        sys.used_memory() as f64 / 1e9,
        sys.available_memory() as f64 / 1e9,
        sys.total_memory() as f64 / 1e9
    );
    for line in memory_stat.lines() {
        memory_list_report.push(Line::from(vec![Span::raw(line)]));
    }

    let memory_paragraph = Paragraph::new(memory_list_report)
        .block(Block::default().borders(Borders::ALL).title("Memory").borders(Borders::ALL))
        .alignment(ratatui::layout::Alignment::Left);
    f.render_widget(memory_paragraph, chunk);
}

pub fn display_disks(sys: &mut System, f: &mut Frame<'_, CrosstermBackend<Stdout>>, chunk: Rect){
    let mut disk_list = vec![];
    for disk in sys.disks_mut() {
        let disk_stat = format!(
            "Name: {:?}\nKind: {:?}\nAvailable Space: {:.2} GB\nTotal Space: {:.2} GB\n ", 
            disk.name(), 
            disk.kind(), 
            disk.available_space() as f64 / 1e9,
            disk.total_space() as f64 / 1e9
        );
        for line in disk_stat.lines() {
            disk_list.push(Line::from(vec![Span::raw(line.to_string())]));
        }
    }
    let disk_paragraph = Paragraph::new(disk_list)
        .block((Block::default().borders(Borders::ALL)).title("Disks").borders(Borders::ALL))
        .alignment(ratatui::layout::Alignment::Left);
    f.render_widget(disk_paragraph, chunk);
}

pub fn display_processes(sys: &System, f: &mut Frame<'_, CrosstermBackend<Stdout>>, chunk: Rect, app: &mut App){
    let mut processes_list = vec![];
    for (pid, process) in sys.processes() {
        let id = format!("[{:7}]", pid);
        let total_read = format!("Total Read: {:8.2} MB", process.disk_usage().total_read_bytes as f64 / 1e6);
        let total_written = format!("Total Written: {:8.2} MB", process.disk_usage().total_written_bytes as f64 / 1e6);

        let processes_stat = format!(
            "{:7} {:35} {:23} | {:23}", 
            id, 
            process.name(),
            total_read,
            total_written
        );
        processes_list.push(Line::from(vec![Span::raw(processes_stat)])); 
    }
    app.vertical_scroll_state = app.vertical_scroll_state.content_length(processes_list.len() as u16);
    let processes_paragraph = Paragraph::new(processes_list.clone())
        .block((Block::default().borders(Borders::ALL)).title("Processes").borders(Borders::ALL))
        .alignment(ratatui::layout::Alignment::Left)
        .scroll((app.vertical_scroll as u16, 0));

    f.render_widget(processes_paragraph, chunk);
    f.render_stateful_widget(
        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .symbols(scrollbar::VERTICAL)
            .begin_symbol(None)
            .track_symbol(None)
            .end_symbol(None),
        chunk.inner(&Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut app.vertical_scroll_state,
    );
}

pub fn get_per_second_list(total_tick_list: &VecDeque<f32>, selected_seconds: i32) -> Vec<f32> {
    let mut tick_list = Vec::new();
    for i in (total_tick_list.len() - (selected_seconds * 4) as usize)..total_tick_list.len() {
        tick_list.push(total_tick_list[i]);
    }

    let mut second_list = Vec::new();
    for chunk in tick_list.chunks(4) {
        let sum: f32 = chunk.iter().sum();
        let average = sum / 4.;
        second_list.push(average);
    }
    second_list
}

pub fn create_html_report(per_second_list: &[f32]) {
    let current_datetime = Local::now();
    let date = current_datetime.format("%Y-%m-%d");

    let mut report = format!(
r#"
<style>
    table, td, th {{
        border: 1px solid #000000;
        border-collapse: collapse;
        padding: 4px 8px;
        text-align: center;
    }}
    table {{
        float: left;
        margin: 0px 2px;
    }}
</style>
<h3>Average CPU Report</h3>
<p>Date: {}</p>
<table>
    <tr>
        <th>Time</th>
        <th>Average CPU</th>
    </tr>
"#, date
    );
    for (i, percentage) in (per_second_list).iter().enumerate() {
        report.push_str(
            format!(
                "\t<tr>\n\t\t<td>{}</td>\n\t\t<td>{:.2}%</td>\n\t</tr>\n",
                subtract_seconds((per_second_list.len() - (i+1)) as i64),
                percentage
            ).as_str()
        );
        if ((i + 1) % 20 == 0) && (i + 1 < per_second_list.len()){
            report.push_str(
r#"</table>
<table>
    <tr>
        <th>Time</th>
        <th>Average CPU</th>
    </tr>
"#
            );
        }
    }
    
    report.push_str("</table>");
    
    let filename = format!("avg_cpu_report_{}.html", current_datetime.format("%H-%M-%S"));
    
    let mut file = File::create(&filename).expect("none");
    let _ = file.write_all(report.as_bytes());
}

fn subtract_seconds(seconds: i64) -> String {
    let now = Local::now();
    let current_hour = now.hour();
    let current_minute = now.minute();
    let current_second = now.second();

    let total_seconds = current_hour * 3600 + current_minute * 60 + current_second;
    
    let new_total_seconds = if seconds <= total_seconds as i64 {
        total_seconds - seconds as u32
    } else {
        (86400 + total_seconds - seconds as u32) % 86400  // 86400 seconds in a day
    };

    let new_hour = new_total_seconds / 3600;
    let new_minute = (new_total_seconds % 3600) / 60;
    let new_second = new_total_seconds % 60;

    format!("{:02}:{:02}:{:02}", new_hour, new_minute, new_second)
}

pub fn display_report_options(f: &mut Frame<'_, CrosstermBackend<Stdout>>, chunk: Rect, report_options: &[i32], current_report_option: &mut ListState) {
    let items: Vec<_> = report_options.iter()
    .map(|&s| {
        ListItem::new(format!("Last {} seconds", s))
    })
    .collect();

    let report_menu = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Average CPU Usage Report"))
        .highlight_style(Style::default().fg(Color::Yellow))
        .highlight_symbol(">");
    f.render_stateful_widget(report_menu, chunk, current_report_option);
}

pub fn display_report_status(f: &mut Frame<'_, CrosstermBackend<Stdout>>, chunk: Rect, report_status: &str) {
    let report_status_paragraph = Paragraph::new(report_status)
        .block(Block::default().borders(Borders::ALL).title("Report Status").borders(Borders::ALL))
        .alignment(ratatui::layout::Alignment::Left);
    f.render_widget(report_status_paragraph, chunk);
}