use std::io;
use std::time::Duration;

use color_eyre::Result;
use crossterm::{
    event::{Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, List, ListItem, Paragraph, Row, Sparkline, Table, Tabs, Wrap},
    Terminal,
};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

// -- Channel messages from background RPC task -------------------------

const RPC_URL: &str = "http://127.0.0.1:19443";

enum NodeEvent {
    Connected {
        block_height: u64,
        peer_count: u64,
        network: String,
        sync_status: String,
    },
    PeersUpdated(Vec<PeerInfo>),
    MiningInfo {
        hashrate: f64,
        difficulty: f64,
    },
    CommandOutput(Vec<Line<'static>>),
    WalletInfo {
        address: String,
        balance: String,
    },
    Disconnected,
}

/// Generic JSON-RPC POST helper. Returns the `result` field on success.
async fn rpc_post(
    method: &str,
    params: serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let resp = client
        .post(RPC_URL)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        }))
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    if let Some(error) = json.get("error").and_then(|e| e.as_object()) {
        let msg = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        return Err(format!("RPC error: {}", msg));
    }

    Ok(json["result"].clone())
}

#[derive(Debug, Clone)]
struct PeerInfo {
    address: String,
    height: u64,
    direction: String,
}

// -- Input Mode ---------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Normal,
    Editing,
}

// -- Model (owns all state, updated by events) --------------------------

struct Model {
    messages: Vec<Line<'static>>,
    input: Input,
    input_mode: InputMode,
    should_quit: bool,
    active_tab: usize,
    rpc_tx: mpsc::Sender<NodeEvent>,
    // Node data (updated via mpsc channel)
    online: bool,
    block_height: u64,
    peer_count: u64,
    network: String,
    sync_status: String,
    peers: Vec<PeerInfo>,
    // Dashboard data
    hashrate: f64,
    difficulty: f64,
    hashrate_history: Vec<u64>,
    // Wallet data
    wallet_address: String,
    wallet_balance: String,
}

impl Model {
    fn new(rpc_tx: mpsc::Sender<NodeEvent>) -> Self {
        let hashrate_history: Vec<u64> = (0..50)
            .map(|i| 30u64 + (i as f64 * 0.5).sin().abs() as u64 * 20 + (i % 7) as u64 * 3)
            .collect();

        Self {
            rpc_tx,
            messages: vec![
                Line::styled("BitQuan Dashboard v0.1.0", Color::Cyan),
                Line::from(""),
                Line::from(
                    Span::raw("  Press ")
                        + Span::styled("i", Style::default().bold())
                        + Span::raw(" to type, ")
                        + Span::styled("Ctrl+C", Style::default().bold())
                        + Span::raw(" to quit"),
                ),
                Line::from(
                    Span::raw("  Type ")
                        + Span::styled("help", Style::default().bold())
                        + Span::raw(" for commands, ")
                        + Span::styled("1/2/3", Style::default().bold())
                        + Span::raw(" to switch tabs"),
                ),
                Line::from(""),
            ],
            input: Input::default(),
            input_mode: InputMode::Normal,
            should_quit: false,
            active_tab: 0,
            online: false,
            block_height: 0,
            peer_count: 0,
            network: "testnet".to_string(),
            sync_status: "offline".to_string(),
            peers: vec![],
            hashrate: 0.0,
            difficulty: 0.0,
            hashrate_history,
            wallet_address: String::new(),
            wallet_balance: "0".to_string(),
        }
    }

    fn submit_command(&mut self) {
        let cmd = self.input.value().trim().to_string();
        if cmd.is_empty() {
            return;
        }

        self.messages
            .push(Line::styled(format!("> {}", cmd), Color::Cyan));
        self.input.reset();

        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let command = parts.first().copied().unwrap_or("");
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        match command {
            "help" => {
                for line in Self::help_text() {
                    self.messages.push(line);
                }
                self.messages.push(Line::from(""));
            }
            "status" | "getinfo" | "mining" | "tx" | "send" | "history" => {
                self.messages
                    .push(Line::styled("  Fetching...", Color::DarkGray));
                let tx = self.rpc_tx.clone();
                let cmd_owned = command.to_string();
                let _ = tokio::spawn(async move {
                    let lines = match cmd_owned.as_str() {
                        "status" | "getinfo" => rpc_status().await,
                        "mining" => rpc_mining().await,
                        "tx" => rpc_tx_detail(&args).await,
                        "send" => rpc_send(&args).await,
                        "history" => rpc_history().await,
                        _ => vec![],
                    };
                    let _ = tx.send(NodeEvent::CommandOutput(lines)).await;
                });
            }
            "peers" => {
                if self.peers.is_empty() {
                    self.messages.push(Line::from("  No peers connected."));
                } else {
                    self.messages.push(Line::styled(
                        format!("  {} peers connected:", self.peers.len()),
                        Color::Cyan,
                    ));
                    for peer in &self.peers {
                        self.messages.push(Line::from(format!(
                            "    {}  height={}  {}",
                            peer.address, peer.height, peer.direction
                        )));
                    }
                }
                self.messages.push(Line::from(""));
            }
            "clear" => {
                self.messages.clear();
                self.messages.push(Line::from("  Log cleared."));
            }
            "quit" | "exit" => {
                self.should_quit = true;
                self.messages.push(Line::from("  Goodbye."));
            }
            _ => {
                self.messages.push(Line::styled(
                    format!("  Unknown command: {}", command),
                    Color::Red,
                ));
                self.messages
                    .push(Line::from("  Type help for available commands."));
            }
        }
    }

    fn help_text() -> Vec<Line<'static>> {
        vec![
            Line::styled("Available Commands:", Style::default().bold()),
            Line::from(""),
            Line::from(
                Span::raw("  ")
                    + Span::styled("help", Color::Yellow)
                    + Span::raw("                    Show this help"),
            ),
            Line::from(
                Span::raw("  ")
                    + Span::styled("status", Color::Yellow)
                    + Span::raw("                  Node info (RPC)"),
            ),
            Line::from(
                Span::raw("  ")
                    + Span::styled("mining", Color::Yellow)
                    + Span::raw("                  Mining info (RPC)"),
            ),
            Line::from(
                Span::raw("  ")
                    + Span::styled("peers", Color::Yellow)
                    + Span::raw("                    Connected peers"),
            ),
            Line::from(
                Span::raw("  ")
                    + Span::styled("tx <txid>", Color::Yellow)
                    + Span::raw("              Transaction (RPC)"),
            ),
            Line::from(
                Span::raw("  ")
                    + Span::styled("send <addr> <amt>", Color::Yellow)
                    + Span::raw("       Send funds (RPC)"),
            ),
            Line::from(
                Span::raw("  ")
                    + Span::styled("history", Color::Yellow)
                    + Span::raw("                  Wallet history (RPC)"),
            ),
            Line::from(
                Span::raw("  ")
                    + Span::styled("clear", Color::Yellow)
                    + Span::raw("                    Clear log"),
            ),
            Line::from(
                Span::raw("  ")
                    + Span::styled("quit", Color::Yellow)
                    + Span::raw("                     Exit"),
            ),
            Line::from(""),
            Line::from(
                Span::raw("  Keys: ")
                    + Span::styled("1/2/3", Color::Green)
                    + Span::raw(" tabs  ")
                    + Span::styled("Tab", Color::Green)
                    + Span::raw(" cycle"),
            ),
        ]
    }
}

// -- Event handling -----------------------------------------------------

fn handle_node_event(model: &mut Model, event: NodeEvent) {
    match event {
        NodeEvent::Connected {
            block_height,
            peer_count,
            network,
            sync_status,
        } => {
            let was_offline = !model.online;
            model.online = true;
            model.block_height = block_height;
            model.peer_count = peer_count;
            model.network = network;
            model.sync_status = sync_status;
            if was_offline {
                model.messages.push(Line::styled(
                    "  [node] Connected to BitQuan node",
                    Color::Green,
                ));
            }
        }
        NodeEvent::PeersUpdated(peers) => {
            model.peers = peers;
        }
        NodeEvent::MiningInfo {
            hashrate,
            difficulty,
        } => {
            model.hashrate = hashrate;
            model.difficulty = difficulty;
            // Push to sparkline history (keep 50 points)
            let h = hashrate as u64;
            model.hashrate_history.push(h);
            if model.hashrate_history.len() > 50 {
                let _ = model.hashrate_history.remove(0);
            }
        }
        NodeEvent::Disconnected => {
            if model.online {
                model
                    .messages
                    .push(Line::styled("  [node] Disconnected", Color::Red));
            }
            model.online = false;
            model.sync_status = "offline".to_string();
        }
        NodeEvent::CommandOutput(lines) => {
            for line in lines {
                model.messages.push(line);
            }
            model.messages.push(Line::from(""));
        }
        NodeEvent::WalletInfo { address, balance } => {
            if !address.is_empty() {
                model.wallet_address = address;
            }
            model.wallet_balance = balance;
        }
    }
}

fn update(model: &mut Model, event: Event) {
    if let Event::Key(key) = event {
        if key.kind != KeyEventKind::Press {
            return;
        }

        // Ctrl+C always quits
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            model.should_quit = true;
            return;
        }

        match model.input_mode {
            InputMode::Normal => match (key.code, key.modifiers) {
                (KeyCode::Char('q') | KeyCode::Esc, _) => {
                    model.should_quit = true;
                }
                (KeyCode::Char('i'), KeyModifiers::NONE) => {
                    model.input_mode = InputMode::Editing;
                }
                (KeyCode::Char('1'), KeyModifiers::NONE) => {
                    model.active_tab = 0;
                }
                (KeyCode::Char('2'), KeyModifiers::NONE) => {
                    model.active_tab = 1;
                }
                (KeyCode::Char('3'), KeyModifiers::NONE) => {
                    model.active_tab = 2;
                }
                (KeyCode::Tab, KeyModifiers::NONE) => {
                    model.active_tab = (model.active_tab + 1) % 3;
                }
                (KeyCode::BackTab, _) => {
                    model.active_tab = (model.active_tab + 2) % 3;
                }
                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Enter => model.submit_command(),
                KeyCode::Esc => model.input_mode = InputMode::Normal,
                _ => {
                    let _ = model.input.handle_event(&event);
                }
            },
        }
    }
}

// -- View (pure rendering, no side effects) -----------------------------

fn view(model: &Model, frame: &mut ratatui::Frame) {
    let outer = Layout::vertical([
        Constraint::Length(3), // Tabs
        Constraint::Min(10),   // Main content
        Constraint::Length(8), // Bottom panels
        Constraint::Length(1), // Status bar
        Constraint::Length(3), // Input
    ])
    .split(frame.area());

    // -- Tabs -----------------------------------------------------------
    let tabs = Tabs::new(vec![
        Line::from("Node"),
        Line::from("Wallet"),
        Line::from("Network"),
    ])
    .block(Block::bordered().title(" BitQuan Dashboard "))
    .select(model.active_tab)
    .style(Style::default().fg(Color::DarkGray))
    .highlight_style(Style::default().fg(Color::Yellow).bold());
    frame.render_widget(tabs, outer[0]);

    // -- Top: Log (60%) + Right panel (40%) ----------------------------
    let top = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(outer[1]);

    // System Log (always visible)
    let logs = Paragraph::new(model.messages.clone())
        .block(
            Block::bordered()
                .title(" System Log ")
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(logs, top[0]);

    // Right panel: content depends on active tab
    match model.active_tab {
        0 => render_peers_panel(model, frame, top[1]),
        1 => render_wallet_panel(model, frame, top[1]),
        2 => render_network_panel(model, frame, top[1]),
        _ => {}
    }

    // -- Bottom: Left (50%) + Right (50%) ------------------------------
    let bottom = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[2]);

    match model.active_tab {
        0 => {
            render_hashrate_panel(model, frame, bottom[0]);
            render_blocks_panel(model, frame, bottom[1]);
        }
        1 => {
            render_transactions_panel(frame, bottom[0]);
            render_utxo_panel(frame, bottom[1]);
        }
        2 => {
            render_mempool_panel(frame, bottom[0]);
            render_consensus_panel(model, frame, bottom[1]);
        }
        _ => {}
    }

    // -- Status Bar -----------------------------------------------------
    let online_color = if model.online {
        Color::Green
    } else {
        Color::Red
    };
    let online_text = if model.online { "Online" } else { "Offline" };
    let sync_display = if model.sync_status == "synced" {
        Span::styled("Synced", Color::Green)
    } else if model.sync_status == "syncing" {
        Span::styled("Syncing...", Color::Yellow)
    } else {
        Span::styled("Offline", Color::Red)
    };
    let status_bar = Paragraph::new(Line::from(vec![
        " BitQuan ".bold(),
        " | ".into(),
        Span::styled(format!(" Blocks: {} ", model.block_height), Color::Cyan),
        " | ".into(),
        Span::styled(format!(" Peers: {} ", model.peer_count), Color::White),
        " | ".into(),
        Span::styled(format!(" Diff: {:.2} ", model.difficulty), Color::White),
        " | ".into(),
        sync_display,
        " | ".into(),
        Span::styled(
            format!(" {} ", online_text),
            Style::default().fg(online_color).bold(),
        ),
    ]))
    .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status_bar, outer[3]);

    // -- Input Bar ------------------------------------------------------
    render_input_bar(model, frame, outer[4]);
}

// -- Tab: Node panels --------------------------------------------------

fn render_peers_panel(model: &Model, frame: &mut ratatui::Frame, area: Rect) {
    let rows: Vec<Row> = if model.peers.is_empty() {
        vec![Row::new(["-- no peers --", "", ""])]
    } else {
        model
            .peers
            .iter()
            .map(|p| {
                let style = if p.direction == "inbound" {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };
                let height_str = p.height.to_string();
                Row::new([p.address.clone(), height_str, p.direction.clone()]).style(style)
            })
            .collect()
    };

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(45),
            Constraint::Percentage(25),
            Constraint::Percentage(30),
        ],
    )
    .header(
        Row::new(["Address", "Height", "Direction"])
            .style(Style::default().bold().fg(Color::Cyan))
            .bottom_margin(1),
    )
    .block(
        Block::bordered()
            .title(format!(" Peers ({}) ", model.peer_count))
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(table, area);
}

fn render_hashrate_panel(model: &Model, frame: &mut ratatui::Frame, area: Rect) {
    let hr_display = if model.hashrate > 1_000_000.0 {
        format!("{:.1} MH/s", model.hashrate / 1_000_000.0)
    } else if model.hashrate > 1_000.0 {
        format!("{:.1} KH/s", model.hashrate / 1_000.0)
    } else {
        format!("{:.0} H/s", model.hashrate)
    };

    let spark_max = model
        .hashrate_history
        .iter()
        .copied()
        .max()
        .unwrap_or(80)
        .max(1);

    let sparkline = Sparkline::default()
        .block(
            Block::bordered()
                .title(format!(" Hashrate ({}) ", hr_display))
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .data(&model.hashrate_history)
        .max(spark_max);
    frame.render_widget(sparkline, area);
}

fn render_blocks_panel(model: &Model, frame: &mut ratatui::Frame, area: Rect) {
    let items: Vec<ListItem> = (0..5)
        .map(|i| {
            let height = model.block_height.saturating_sub(i as u64);
            let style = if i == 0 {
                Style::default().fg(Color::Green).bold()
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(
                format!("  #{}  ({} tx)", height, 12 + i * 3),
                style,
            )))
        })
        .collect();

    let list = List::new(items).block(
        Block::bordered()
            .title(format!(" Latest Blocks ({}) ", model.block_height))
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(list, area);
}

// -- Tab: Wallet panels ------------------------------------------------

fn render_wallet_panel(model: &Model, frame: &mut ratatui::Frame, area: Rect) {
    let addr = if model.wallet_address.is_empty() {
        "(no wallet loaded)".to_string()
    } else {
        model.wallet_address.clone()
    };
    let balance = &model.wallet_balance;

    let wallet = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled("  Address:", Style::default().bold())),
        Line::from(format!("  {}", addr)),
        Line::from(""),
        Line::from(Span::styled("  Balance:", Style::default().bold())),
        Line::from(Span::styled(format!("  {} BQ", balance), Color::Green)),
    ])
    .block(
        Block::bordered()
            .title(" Wallet Info ")
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(wallet, area);
}

fn render_transactions_panel(frame: &mut ratatui::Frame, area: Rect) {
    let txs = List::new(vec![
        ListItem::new(Line::from(Span::styled("  recv  +50.000 BQ", Color::Green))),
        ListItem::new(Line::from(Span::raw("  send  -12.500 BQ"))),
        ListItem::new(Line::from(Span::styled(
            "  recv  +200.000 BQ",
            Color::Green,
        ))),
        ListItem::new(Line::from(Span::raw("  send   -1.234 BQ"))),
    ])
    .block(
        Block::bordered()
            .title(" Recent Transactions ")
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(txs, area);
}

fn render_utxo_panel(frame: &mut ratatui::Frame, area: Rect) {
    let utxo = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  UTXO Pool",
            Style::default().bold().fg(Color::Cyan),
        )),
        Line::from(""),
        Line::from("  Total UTXOs:    3"),
        Line::from(Span::raw("  Total Value:    ") + Span::styled("1,234.56 BQ", Color::Green)),
        Line::from("  Largest:        500.00 BQ"),
        Line::from("  Smallest:       0.12 BQ"),
    ])
    .block(
        Block::bordered()
            .title(" UTXO Pool ")
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(utxo, area);
}

// -- Tab: Network panels -----------------------------------------------

fn render_network_panel(model: &Model, frame: &mut ratatui::Frame, area: Rect) {
    let info = Paragraph::new(vec![
        Line::from(""),
        Line::from(format!("  Protocol:  {}", "PQC-Blake3")),
        Line::from(format!("  Network:   {}", model.network)),
        Line::from(format!("  Peers:     {}", model.peer_count)),
        Line::from(format!("  Height:    {}", model.block_height)),
        Line::from(""),
        Line::from(format!("  Status:    {}", model.sync_status)),
    ])
    .block(
        Block::bordered()
            .title(" Network Info ")
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(info, area);
}

fn render_mempool_panel(frame: &mut ratatui::Frame, area: Rect) {
    let mempool = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Mempool Status",
            Style::default().bold().fg(Color::Cyan),
        )),
        Line::from(""),
        Line::from("  Pending TXs:    42"),
        Line::from("  Total Fees:     0.0089 BQ"),
        Line::from("  Min Fee Rate:   0.0001 BQ/kB"),
        Line::from("  Max TX Age:     12 blocks"),
    ])
    .block(
        Block::bordered()
            .title(" Mempool ")
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(mempool, area);
}

fn render_consensus_panel(model: &Model, frame: &mut ratatui::Frame, area: Rect) {
    let consensus = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Consensus",
            Style::default().bold().fg(Color::Cyan),
        )),
        Line::from(""),
        Line::from(format!("  Algorithm:      {}", "PQC-Blake3")),
        Line::from(format!("  Block Time:     {}s", 120)),
        Line::from(format!("  Current Height: {}", model.block_height)),
        Line::from(format!("  Difficulty:     {:.4}", model.difficulty)),
    ])
    .block(
        Block::bordered()
            .title(" Consensus ")
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(consensus, area);
}

// -- Input bar (shared across all tabs) ---------------------------------

fn render_input_bar(model: &Model, frame: &mut ratatui::Frame, area: Rect) {
    let block = Block::bordered()
        .border_style(if model.input_mode == InputMode::Editing {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        })
        .title(if model.input_mode == InputMode::Editing {
            Line::from(Span::styled(" Command (Esc to cancel) ", Color::Yellow))
        } else {
            Line::from(" Command (press i to type) ")
        });
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let width = inner.width.max(3) - 2;
    let scroll = model.input.visual_scroll(width as usize);
    let input_text = Paragraph::new(model.input.value()).scroll((0, scroll as u16));
    frame.render_widget(input_text, inner);

    if model.input_mode == InputMode::Editing {
        let cursor_x = model.input.visual_cursor().max(scroll) - scroll + 1;
        frame.set_cursor_position((inner.x + cursor_x as u16, inner.y));
    }
}

// -- Async RPC command handlers (spawned from submit_command) ----------

async fn rpc_status() -> Vec<Line<'static>> {
    match rpc_post("getblockchaininfo", serde_json::json!([])).await {
        Ok(result) => {
            let mut lines = vec![Line::styled("  Blockchain Info", Style::default().bold())];
            if let Some(obj) = result.as_object() {
                let mut entries: Vec<(String, String)> = obj
                    .iter()
                    .map(|(k, v)| (k.clone(), format!("{}", v)))
                    .collect();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                for (key, val) in entries {
                    lines.push(Line::from(format!("    {}: {}", key, val)));
                }
            } else {
                lines.push(Line::from(format!("  {}", result)));
            }
            lines
        }
        Err(e) => vec![Line::styled(format!("  Error: {}", e), Color::Red)],
    }
}

async fn rpc_mining() -> Vec<Line<'static>> {
    match rpc_post("getmininginfo", serde_json::json!([])).await {
        Ok(result) => {
            let blocks = result["blocks"].as_u64().unwrap_or(0);
            let difficulty = result["difficulty"].as_f64().unwrap_or(0.0);
            let hashrate = result["networkhashps"].as_f64().unwrap_or(0.0);

            let hr = if hashrate > 1_000_000.0 {
                format!("{:.2} MH/s", hashrate / 1_000_000.0)
            } else if hashrate > 1_000.0 {
                format!("{:.2} KH/s", hashrate / 1_000.0)
            } else {
                format!("{:.0} H/s", hashrate)
            };

            vec![
                Line::styled("  Mining Info", Style::default().bold()),
                Line::from(format!("    Blocks:     {}", blocks)),
                Line::from(format!("    Difficulty: {:.4}", difficulty)),
                Line::from(format!("    Hashrate:   {}", hr)),
            ]
        }
        Err(e) => vec![Line::styled(format!("  Error: {}", e), Color::Red)],
    }
}

async fn rpc_tx_detail(args: &[String]) -> Vec<Line<'static>> {
    let txid = match args.first() {
        Some(id) => id.clone(),
        None => {
            return vec![Line::styled("  Usage: tx <txid>", Color::Yellow)];
        }
    };

    match rpc_post("gettransaction", serde_json::json!([txid])).await {
        Ok(result) => {
            let mut lines = vec![Line::styled(
                format!("  Transaction {}", &txid[..8.min(txid.len())]),
                Style::default().bold(),
            )];
            if let Some(obj) = result.as_object() {
                for (key, val) in obj {
                    lines.push(Line::from(format!("    {}: {}", key, val)));
                }
            } else {
                lines.push(Line::from(format!("  {}", result)));
            }
            lines
        }
        Err(e) => vec![Line::styled(format!("  Error: {}", e), Color::Red)],
    }
}

async fn rpc_send(args: &[String]) -> Vec<Line<'static>> {
    if args.len() < 2 {
        return vec![Line::styled(
            "  Usage: send <address> <amount>",
            Color::Yellow,
        )];
    }

    let address = args[0].clone();
    let amount = args[1].clone();

    match rpc_post("sendtoaddress", serde_json::json!([address, amount])).await {
        Ok(result) => {
            let txid = result.as_str().unwrap_or("unknown");
            vec![
                Line::styled("  Transaction sent!", Color::Green),
                Line::from(format!("  TXID: {}", txid)),
            ]
        }
        Err(e) => vec![Line::styled(format!("  Send failed: {}", e), Color::Red)],
    }
}

async fn rpc_history() -> Vec<Line<'static>> {
    match rpc_post("listtransactions", serde_json::json!([])).await {
        Ok(result) => {
            if let Some(arr) = result.as_array() {
                let mut lines = vec![Line::styled(
                    format!("  {} transactions:", arr.len()),
                    Color::Cyan,
                )];
                for (i, tx) in arr.iter().take(10).enumerate() {
                    let txid = tx["txid"].as_str().unwrap_or("?");
                    let short = &txid[..8.min(txid.len())];
                    let confirmations = tx["confirmations"].as_u64().unwrap_or(0);
                    let status = if confirmations >= 6 {
                        Color::Green
                    } else {
                        Color::Yellow
                    };
                    lines.push(Line::from(
                        Span::raw(format!(
                            "    #{} {}  [{} conf]",
                            i + 1,
                            short,
                            confirmations
                        ))
                        .style(status),
                    ));
                }
                if arr.len() > 10 {
                    lines.push(Line::from(format!("    ... and {} more", arr.len() - 10)));
                }
                lines
            } else {
                vec![Line::from("  No transactions found.")]
            }
        }
        Err(_) => vec![
            Line::from("  Wallet history: not yet available on RPC."),
            Line::from("  (listtransactions RPC endpoint pending)"),
        ],
    }
}

// -- Background RPC task (runs in tokio::spawn) -------------------------

async fn node_rpc_task(tx: mpsc::Sender<NodeEvent>) {
    let rpc_url = "http://127.0.0.1:19443";
    let client = reqwest::Client::new();

    loop {
        // Fetch network status (block height, peers, sync)
        match fetch_network_status(&client, rpc_url).await {
            Ok(info) => {
                let _ = tx
                    .send(NodeEvent::Connected {
                        block_height: info.block_height,
                        peer_count: info.peer_count,
                        network: info.network,
                        sync_status: info.sync_status,
                    })
                    .await;
            }
            Err(_) => {
                let _ = tx.send(NodeEvent::Disconnected).await;
            }
        }

        // Fetch mining info (hashrate, difficulty)
        match fetch_mining_info(&client, rpc_url).await {
            Ok(info) => {
                let _ = tx
                    .send(NodeEvent::MiningInfo {
                        hashrate: info.hashrate,
                        difficulty: info.difficulty,
                    })
                    .await;
            }
            Err(_) => {
                // Mining info fetch failed, skip silently
            }
        }

        // Fetch peer list
        match fetch_peer_list(&client, rpc_url).await {
            Ok(peers) => {
                let _ = tx.send(NodeEvent::PeersUpdated(peers)).await;
            }
            Err(_) => {
                // Peer list fetch failed, skip silently
            }
        }

        // Fetch wallet info (graceful — RPC may not be
        // implemented yet)
        match fetch_wallet_info(&client, rpc_url).await {
            Ok(info) => {
                let _ = tx
                    .send(NodeEvent::WalletInfo {
                        address: info.address,
                        balance: info.balance,
                    })
                    .await;
            }
            Err(_) => {
                // Wallet RPCs not implemented yet,
                // skip silently
            }
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

struct NetworkInfo {
    block_height: u64,
    peer_count: u64,
    network: String,
    sync_status: String,
}

async fn fetch_network_status(
    client: &reqwest::Client,
    rpc_url: &str,
) -> std::result::Result<NetworkInfo, Box<dyn std::error::Error + Send + Sync>> {
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "getnetworkstatus",
            "params": [],
            "id": 1
        }))
        .send()
        .await?;

    let json: serde_json::Value = resp.json().await?;
    let r = &json["result"];

    Ok(NetworkInfo {
        block_height: r["local_height"].as_u64().unwrap_or(0),
        peer_count: r["peers_connected"].as_u64().unwrap_or(0),
        network: "testnet".to_string(),
        sync_status: r["sync_status"].as_str().unwrap_or("unknown").to_string(),
    })
}

struct MiningInfoData {
    hashrate: f64,
    difficulty: f64,
}

async fn fetch_mining_info(
    client: &reqwest::Client,
    rpc_url: &str,
) -> std::result::Result<MiningInfoData, Box<dyn std::error::Error + Send + Sync>> {
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "getmininginfo",
            "params": [],
            "id": 3
        }))
        .send()
        .await?;

    let json: serde_json::Value = resp.json().await?;
    let r = &json["result"];

    Ok(MiningInfoData {
        hashrate: r["networkhashps"].as_f64().unwrap_or(0.0),
        difficulty: r["difficulty"].as_f64().unwrap_or(0.0),
    })
}

async fn fetch_peer_list(
    client: &reqwest::Client,
    rpc_url: &str,
) -> std::result::Result<Vec<PeerInfo>, Box<dyn std::error::Error + Send + Sync>> {
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "getpeerinfo",
            "params": [],
            "id": 4
        }))
        .send()
        .await?;

    let json: serde_json::Value = resp.json().await?;

    let peers = json["result"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|p| PeerInfo {
                    address: p["addr"].as_str().unwrap_or("?").to_string(),
                    height: p["synced_height"].as_u64().unwrap_or(0),
                    direction: p["direction"].as_str().unwrap_or("?").to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(peers)
}

async fn fetch_wallet_info(
    client: &reqwest::Client,
    rpc_url: &str,
) -> std::result::Result<WalletRpcInfo, Box<dyn std::error::Error + Send + Sync>> {
    let resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "getbalance",
            "params": [],
            "id": 5
        }))
        .send()
        .await?;

    let json: serde_json::Value = resp.json().await?;
    let r = &json["result"];

    let balance = if r.is_null() {
        "0".to_string()
    } else {
        format!("{}", r)
    };

    // Try to get address too
    let address_resp = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "getnewaddress",
            "params": [],
            "id": 6
        }))
        .send()
        .await;

    let address = match address_resp {
        Ok(resp) => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            json["result"].as_str().unwrap_or("").to_string()
        }
        Err(_) => String::new(),
    };

    Ok(WalletRpcInfo { address, balance })
}

struct WalletRpcInfo {
    address: String,
    balance: String,
}

// -- Main ---------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (rpc_tx, rpc_rx) = mpsc::channel(32);
    let _ = tokio::spawn(node_rpc_task(rpc_tx.clone()));

    let mut model = Model::new(rpc_tx);
    let result = run(&mut terminal, &mut model, rpc_rx).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    model: &mut Model,
    mut rpc_rx: mpsc::Receiver<NodeEvent>,
) -> Result<()> {
    let mut events = crossterm::event::EventStream::new();
    let mut render_interval = tokio::time::interval(Duration::from_millis(250));

    while !model.should_quit {
        tokio::select! {
            _ = render_interval.tick() => {
                let _ =
                    terminal.draw(|frame| view(model, frame));
            }
            Some(Ok(event)) = events.next() => {
                update(model, event);
            }
            Some(node_event) = rpc_rx.recv() => {
                handle_node_event(model, node_event);
            }
        }
    }
    Ok(())
}
