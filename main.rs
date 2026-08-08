use crossterm::event::{self, Event, KeyCode, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    DefaultTerminal,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::Text,
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use std::{io::Result, process::Command};

fn main() -> Result<()> {
    // Enable mouse capture during initialization
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;

    let mut terminal = ratatui::init();
    let res = wifi_menu(&mut terminal);

    // Restore terminal and disable mouse capture
    ratatui::restore();
    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;

    Ok(())
}

fn wifi_menu(terminal: &mut DefaultTerminal) -> Result<WifiMenuResult> {
    let wifis = get_wifi_info();
    let mut upper_list_state = ListState::default();
    upper_list_state.select(Some(0));
    let mut lower_list_state = ListState::default();
    lower_list_state.select(Some(0));
    let mut focus_below = false;

    let mut opt_count_up = 0;
    let mut opt_count_down = 0;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            match &wifis {
                WifiQueryResult::Success(wifi_infos) => {
                    let mut upper_list_items: Vec<ListItem> = Vec::new();
                    upper_list_items.push(ListItem::new("Other Options"));
                    for wifi_info in wifi_infos {
                        upper_list_items.push(
                        	ListItem::new(format!("{}{}", wifi_info.ssid, if wifi_info.active { " (Connected)"} else {""}))
                        )
                    }

                    let lower_list_items = if let Some(x) = upper_list_state.selected() && x != 0 {
                   		vec![
                     ListItem::new(if wifi_infos[x - 1].active {
                     	"Disconnect"
                     } else { "Connect" }),
                     ListItem::new("Try Forget")
                     ]
                    } else {
	                   	vec![
	                        ListItem::new("Wifi Off"),
	                        ListItem::new("Ping google.com"),
	                        ListItem::new("Ping 8.8.8.8"),
	                    ]
                    };

                    opt_count_up = upper_list_items.len();
                    opt_count_down = lower_list_items.len();

                    let main_split = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(60), Constraint::Fill(1)])
                        .split(area);
                    let left_pane = main_split[0];
                    let right_pane = main_split[1];
                    let left_split = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Fill(1), Constraint::Length(5)])
                        .split(left_pane);
                    let upper_pane = left_split[0];
                    let lower_pane = left_split[1];

                    let upper_list = List::new(upper_list_items)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_type(if !focus_below { BorderType::Double } else { BorderType::Plain })
                                .title_alignment(Alignment::Center)
                                .title_top(if !focus_below { " Wifi Networks ".bold() } else { " Wifi Networks ".dim() }),
                        )
                        .highlight_symbol(">> ")
                        .highlight_style(Style::default().bg(Color::White).fg(Color::Black));
                    frame.render_stateful_widget(upper_list, upper_pane, &mut upper_list_state);
                    let lower_list = List::new(lower_list_items)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_type(if focus_below { BorderType::Double } else { BorderType::Plain })
                                .title_alignment(Alignment::Center)
                                .title_top(if focus_below { " Action ".bold() } else { " Action ".dim() }),
                        )
                        .highlight_symbol(">> ")
                        .highlight_style(Style::default().bg(Color::White).fg(Color::Black));
                    frame.render_stateful_widget(lower_list, lower_pane, &mut lower_list_state);

                    let paragraph = Paragraph::new(Text::from(
                    	if let Some(x) = upper_list_state.selected() && x != 0 {
                     let wifi_info = &wifi_infos[x - 1];
	                   		format!("Wifi Active: {}\nSSID: {}\nSignal: {}%\nFreq: {}\nSecurity: {}\nRate: {}", wifi_info.active, wifi_info.ssid, wifi_info.signal, wifi_info.freq, wifi_info.security, wifi_info.rate)
	                    } else {
	                     	format!("Found {} wifi networks.\n\nUse Arrow Keys/j/k to move up/down\n\nTAB to change focus.\n\nPress Enter on Wifi to take action.\n\nQ/Esc to Quit\nR to Reload", wifi_infos.len())
	                    }
                    )).block(
                    	Block::default().borders(Borders::ALL).title_top(" Details ")
                    ).wrap(Wrap { trim: false });
                    frame.render_widget(paragraph, right_pane);
                }
                WifiQueryResult::FailureUnknown => {
                    let paragraph = Paragraph::new(
                        Text::from(
                            "Unable to fetch wifi info\nPress Q/Esc to quit.\nPress R to retry.",
                        )
                        .centered(),
                    )
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title_alignment(Alignment::Center)
                            .title(" Error ".red()),
                    );
                    frame.render_widget(paragraph, area);
                },
                WifiQueryResult::FailureRadioOff => {
               	let paragraph = Paragraph::new(
                    Text::from(
                        "Wifi is off! Turn it on by pressing R.\nPress Q/Esc to quit.",
                    )
                    .centered(),
                )
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title_alignment(Alignment::Center)
                        .title(" Error ".red()),
                );
                frame.render_widget(paragraph, area);
                }
            }
        })?;

        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    return Ok(WifiMenuResult {
                        next_win: Win::Exit,
                        operation: WifiOperation::Nothing,
                        ssid: String::new(),
                    });
                }
                KeyCode::Char('r') => {
                    return Ok(WifiMenuResult {
                        next_win: Win::WifiMenu,
                        operation: if let WifiQueryResult::FailureRadioOff = wifis {
                            WifiOperation::RadioOn
                        } else {
                            WifiOperation::Nothing
                        },
                        ssid: String::new(),
                    });
                }
                KeyCode::Up | KeyCode::Char('j') => {
                    if focus_below {
                        if let Some(x) = lower_list_state.selected() {
                            lower_list_state.select(Some(
                                x.checked_sub(1).unwrap_or(opt_count_down.saturating_sub(1)),
                            ));
                        }
                    } else {
                        if let Some(x) = upper_list_state.selected() {
                            upper_list_state.select(Some(
                                x.checked_sub(1).unwrap_or(opt_count_up.saturating_sub(1)),
                            ));
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('k') => {
                    if focus_below {
                        if let Some(x) = lower_list_state.selected() {
                            lower_list_state.select(Some(if x + 1 == opt_count_down {
                                0
                            } else {
                                x + 1
                            }));
                        }
                    } else {
                        if let Some(x) = upper_list_state.selected() {
                            upper_list_state.select(Some(if x + 1 == opt_count_up {
                                0
                            } else {
                                x + 1
                            }));
                        }
                    }
                }
                KeyCode::Enter => {
                    if focus_below {
                        unimplemented!()
                    } else {
                        focus_below = true;
                    }
                }
                KeyCode::Tab => focus_below = !focus_below,
                _ => (),
            },
            _ => (),
        }
    }
}

struct WifiInfo {
    active: bool,
    ssid: String,
    signal: String,
    freq: String,
    rate: String,
    security: String,
}

pub enum Win {
    WifiMenu,
    WifiPasswordMenu,
    PingMenu1,
    PingMenu2,
    WifiError,
    Exit,
}

pub enum WifiOperation {
    Disconnect,
    RadioOff,
    RadioOn,
    Forget,
    Connect,
    Nothing,
}

struct WifiMenuResult {
    pub next_win: Win,
    pub operation: WifiOperation,
    pub ssid: String,
}

enum WifiQueryResult {
    Success(Vec<WifiInfo>),
    FailureRadioOff,
    FailureUnknown,
}

fn get_wifi_info() -> WifiQueryResult {
    let cmd_out = Command::new("nmcli")
        .args([
            "-t",
            "-f",
            "active,signal,security,freq,rate,ssid",
            "device",
            "wifi",
        ])
        .output();

    match cmd_out {
        Ok(cmd_out2) => {
            if cmd_out2.status.success() {
                let output = String::from_utf8_lossy(&cmd_out2.stdout);
                let mut result = Vec::new();
                for line in output.lines() {
                    let parts: Vec<&str> = line.split(":").collect();
                    if parts.len() >= 6 {
                        result.push(WifiInfo {
                            active: parts[0] == "yes",
                            ssid: parts[5..].join(":"),
                            signal: parts[1].to_string(),
                            freq: parts[3].to_string(),
                            rate: parts[4].to_string(),
                            security: parts[2].to_string(),
                        })
                    }
                }
                WifiQueryResult::Success(result)
            } else {
                WifiQueryResult::FailureUnknown
            }
        }
        _ => WifiQueryResult::FailureUnknown,
    }
}
