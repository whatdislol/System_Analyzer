use std::{io::{self}, thread, time::Duration,collections::VecDeque};
use ratatui::{
    backend::CrosstermBackend,
    widgets::ListState,
    layout::{Layout, Constraint, Direction},
    Terminal
};
use crossterm::{
    event::{self, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use system_analyzer::{display_cpu, display_memory, display_disks, display_network, display_processes, display_battery, 
    create_html_report, get_avg_cpu, get_cpu, get_per_second_list, display_report_options, display_report_status, App};
use sysinfo::{System, SystemExt};

fn main() -> Result<(), io::Error> {
    //setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut sys = System::new_all();

    //setup report variables
    let report_options = [10, 30, 60, 120];
    let mut current_report_option = ListState::default();
    current_report_option.select(Some(0));
    let mut tick = 0;
    let mut avg_cpu_tick_list: VecDeque<f32> = VecDeque::with_capacity(120 * 4);
    let mut report_status = "No report is generated.";

    //setup scrollbars
    let mut app = App::default();

    loop{
        sys.refresh_all();
        tick += 1;

        terminal.draw(|f| {
            //set up chunks
            let size = f.size();

            let chunk_main_column = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(10),
                    Constraint::Percentage(25),
                    Constraint::Percentage(20),
                    Constraint::Percentage(45),
                ])
                .split(size);

            let chunk_net_mem_bat = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(41),
                    Constraint::Percentage(26),
                    Constraint::Percentage(33),
                ])
                .split(chunk_main_column[1]);

            let chunk_rep_disk = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(30),
                    Constraint::Percentage(15),
                    Constraint::Percentage(55),
                ])
                .split(chunk_main_column[2]);

            thread::sleep(Duration::from_secs_f64(0.25));

            //display system information
            let cpu_list = get_cpu(&sys);
            let avg_cpu = get_avg_cpu(&cpu_list);
            display_cpu(cpu_list, avg_cpu, f, chunk_main_column[0]);
            display_network(&sys, f, chunk_net_mem_bat[0]);
            display_memory(&sys, f, chunk_net_mem_bat[1]);
            display_report_status(f, chunk_rep_disk[1], report_status);
            display_disks(&mut sys, f, chunk_rep_disk[2]);
            display_processes(&sys, f, chunk_main_column[3], &mut app);
            let _ = display_battery(f, chunk_net_mem_bat[2]);
            display_report_options(f, chunk_rep_disk[0], &report_options, &mut current_report_option);

            //update avg_cpu_tick_list
            avg_cpu_tick_list.push_back(avg_cpu);
            if avg_cpu_tick_list.len() > 120 * 4 {
                avg_cpu_tick_list.pop_front();
            }
        })?;

        //event listeners
        if event::poll(std::time::Duration::from_millis(10))? {
            if let event::Event::Key(KeyEvent {code, modifiers, .. }) = event::read()? {
                match code {
                    KeyCode::Left | KeyCode::Char('a') => {
                        if let Some(selected_index) = current_report_option.selected() {
                            if selected_index > 0 {
                                current_report_option.select(Some(selected_index - 1));
                            }
                        } else {
                            current_report_option.select(Some(report_options.len() - 1));
                        }
                    }
                    KeyCode::Right | KeyCode::Char('d') => {
                        if let Some(selected_index) = current_report_option.selected() {
                            if selected_index < report_options.len() - 1 {
                                current_report_option.select(Some(selected_index + 1));
                            }
                        } else {
                            current_report_option.select(Some(0));
                        }
                    }
                    
                    KeyCode::Enter => {
                        if let Some(selected_index) = current_report_option.selected() {
                            let selected_seconds = report_options[selected_index];
                            if selected_seconds <= (tick / 4) {
                                //generate HTML report
                                report_status = "Report generated.";
                                let per_second_list = get_per_second_list(&avg_cpu_tick_list, selected_seconds);
                                create_html_report(&per_second_list);
                            } else {
                                report_status = "Report duration is longer than runtime." ;
                            }
                        } else {
                            report_status = "No report is selected.";
                        }
                    },
                    KeyCode::Char('c') if modifiers == KeyModifiers::CONTROL => {
                        let _ = terminal.clear();
                        break;
                    },
                    KeyCode::Down | KeyCode::Char('s') => {
                        app.vertical_scroll = app.vertical_scroll.saturating_add(1);
                        app.vertical_scroll_state = app
                            .vertical_scroll_state
                            .position(app.vertical_scroll as u16);
                    }
                    KeyCode::Up | KeyCode::Char('w') => {
                        app.vertical_scroll = app.vertical_scroll.saturating_sub(1);
                        app.vertical_scroll_state = app
                            .vertical_scroll_state
                            .position(app.vertical_scroll as u16);
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(()) 
}
