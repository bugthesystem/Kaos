//! Kaos Asteroids Client
//!
//! Terminal-based client for the multiplayer Asteroids game.
//! Uses kaos-rudp reliable UDP transport with full reliability:
//! - Sequence numbering
//! - NAK-based retransmission
//! - Ordered delivery
//!
//! Controls:
//! - W/Up: Thrust
//! - A/Left: Rotate left
//! - D/Right: Rotate right
//! - Space: Fire
//! - Q: Quit

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{poll, read, Event, KeyCode, KeyModifiers},
    execute,
    style::{Color, Print, SetForegroundColor, ResetColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use kaos_rudp::{RudpTransport, Transport};
use serde::{Deserialize, Serialize};
use std::io::{stdout, Write};
use std::net::SocketAddr;
use std::time::Duration;

const SERVER_ADDR: &str = "127.0.0.1:7352";

// Protocol messages (must match server)
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum ClientMessage {
    Join { name: String },
    Input { thrust: bool, left: bool, right: bool, fire: bool },
    Leave,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum ServerMessage {
    Welcome { player_id: u64 },
    GameState { state: GameState },
    PlayerDied { player_id: u64, score: i64, killer: String },
    GameOver { score: i64, rank: Option<usize> },
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct GameState {
    tick: u64,
    ships: Vec<ShipState>,
    asteroids: Vec<AsteroidState>,
    bullets: Vec<BulletState>,
    leaderboard: Vec<LeaderboardEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ShipState {
    id: u64,
    name: String,
    x: f64,
    y: f64,
    angle: f64,
    score: i64,
    alive: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AsteroidState {
    id: u64,
    x: f64,
    y: f64,
    size: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct BulletState {
    x: f64,
    y: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LeaderboardEntry {
    name: String,
    score: i64,
}

struct InputState {
    thrust: bool,
    left: bool,
    right: bool,
    fire: bool,
}

fn main() -> std::io::Result<()> {
    // Get player name
    let name = std::env::args().nth(1).unwrap_or_else(|| {
        format!("Pilot{}", std::process::id() % 1000)
    });

    println!("Connecting to {} as '{}' using kaos-rudp...", SERVER_ADDR, name);

    // Parse server address
    let server_addr: SocketAddr = SERVER_ADDR.parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    // Create kaos-rudp transport (full reliability: sequences, NAK, retransmit)
    // Bind to any available port, connect to server
    let bind_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let mut transport = RudpTransport::new(bind_addr, server_addr, 256)?;

    println!("Created RudpTransport (window=256)");

    // Send join message using kaos-rudp
    let join_msg = ClientMessage::Join { name: name.clone() };
    let join_data = serde_json::to_vec(&join_msg)?;
    transport.send(&join_data)?;

    // Wait for welcome
    let mut my_id = 0u64;
    for _ in 0..100 {
        transport.receive(|data| {
            if let Ok(msg) = serde_json::from_slice::<ServerMessage>(data) {
                if let ServerMessage::Welcome { player_id } = msg {
                    my_id = player_id;
                }
            }
        });
        if my_id != 0 {
            println!("Connected! Player ID: {} (kaos-rudp transport)", my_id);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    if my_id == 0 {
        println!("Failed to connect to server");
        return Ok(());
    }

    // Setup terminal
    let mut stdout = stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, Hide)?;

    let mut game_state = GameState::default();
    let mut input = InputState {
        thrust: false,
        left: false,
        right: false,
        fire: false,
    };
    let mut running = true;

    while running {
        // Handle keyboard input
        while poll(Duration::from_millis(0))? {
            if let Event::Key(key) = read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        running = false;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        running = false;
                    }
                    KeyCode::Char('w') | KeyCode::Up => {
                        input.thrust = true;
                    }
                    KeyCode::Char('a') | KeyCode::Left => {
                        input.left = true;
                    }
                    KeyCode::Char('d') | KeyCode::Right => {
                        input.right = true;
                    }
                    KeyCode::Char(' ') => {
                        input.fire = true;
                    }
                    _ => {}
                }
            }
        }

        // Send input via kaos-rudp transport (handles headers, sequences, etc.)
        let input_msg = ClientMessage::Input {
            thrust: input.thrust,
            left: input.left,
            right: input.right,
            fire: input.fire,
        };
        let input_data = serde_json::to_vec(&input_msg)?;
        let _ = transport.send(&input_data);

        // Process ACKs from server (maintains reliability)
        transport.process_acks();

        // Reset input (fire is one-shot, movement is continuous)
        input.fire = false;
        input.thrust = false;
        input.left = false;
        input.right = false;

        // Receive game state via kaos-rudp (ordered delivery)
        transport.receive(|data| {
            if let Ok(msg) = serde_json::from_slice::<ServerMessage>(data) {
                match msg {
                    ServerMessage::GameState { state } => {
                        game_state = state;
                    }
                    ServerMessage::GameOver { score, rank } => {
                        let _ = (score, rank);
                    }
                    _ => {}
                }
            }
        });

        // Render
        render(&mut stdout, &game_state, my_id, &name)?;

        std::thread::sleep(Duration::from_millis(16)); // ~60fps rendering
    }

    // Send leave message via kaos-rudp
    let leave_msg = ClientMessage::Leave;
    let leave_data = serde_json::to_vec(&leave_msg)?;
    let _ = transport.send(&leave_data);

    // Cleanup terminal
    execute!(stdout, Show, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    println!("Thanks for playing!");
    Ok(())
}

fn render(
    stdout: &mut std::io::Stdout,
    state: &GameState,
    my_id: u64,
    my_name: &str,
) -> std::io::Result<()> {
    let (term_width, term_height) = terminal::size()?;
    let game_width = (term_width - 20) as f64; // Leave space for UI
    let game_height = (term_height - 4) as f64;

    // Scale factors
    let scale_x = game_width / 100.0;
    let scale_y = game_height / 50.0;

    execute!(stdout, Clear(ClearType::All))?;

    // Draw border
    execute!(stdout, MoveTo(0, 0), Print("╔"))?;
    for _ in 1..game_width as u16 {
        execute!(stdout, Print("═"))?;
    }
    execute!(stdout, Print("╗"))?;

    for y in 1..game_height as u16 {
        execute!(stdout, MoveTo(0, y), Print("║"))?;
        execute!(stdout, MoveTo(game_width as u16, y), Print("║"))?;
    }

    execute!(stdout, MoveTo(0, game_height as u16), Print("╚"))?;
    for _ in 1..game_width as u16 {
        execute!(stdout, Print("═"))?;
    }
    execute!(stdout, Print("╝"))?;

    // Draw asteroids
    execute!(stdout, SetForegroundColor(Color::DarkYellow))?;
    for asteroid in &state.asteroids {
        let x = ((asteroid.x * scale_x) as u16).min(game_width as u16 - 1).max(1);
        let y = ((asteroid.y * scale_y) as u16).min(game_height as u16 - 1).max(1);
        let char = match asteroid.size {
            3 => '@',
            2 => 'O',
            _ => 'o',
        };
        execute!(stdout, MoveTo(x, y), Print(char))?;
    }

    // Draw bullets
    execute!(stdout, SetForegroundColor(Color::Yellow))?;
    for bullet in &state.bullets {
        let x = ((bullet.x * scale_x) as u16).min(game_width as u16 - 1).max(1);
        let y = ((bullet.y * scale_y) as u16).min(game_height as u16 - 1).max(1);
        execute!(stdout, MoveTo(x, y), Print("·"))?;
    }

    // Draw ships
    for ship in &state.ships {
        let x = ((ship.x * scale_x) as u16).min(game_width as u16 - 1).max(1);
        let y = ((ship.y * scale_y) as u16).min(game_height as u16 - 1).max(1);

        if ship.id == my_id {
            execute!(stdout, SetForegroundColor(Color::Green))?;
        } else {
            execute!(stdout, SetForegroundColor(Color::Cyan))?;
        }

        if ship.alive {
            // Draw ship pointing in direction
            let char = get_ship_char(ship.angle);
            execute!(stdout, MoveTo(x, y), Print(char))?;
        } else {
            execute!(stdout, SetForegroundColor(Color::Red))?;
            execute!(stdout, MoveTo(x, y), Print("X"))?;
        }
    }

    // UI Panel
    let ui_x = game_width as u16 + 2;
    execute!(stdout, ResetColor)?;

    execute!(stdout, MoveTo(ui_x, 1), SetForegroundColor(Color::White))?;
    execute!(stdout, Print("╔═══════════════╗"))?;
    execute!(stdout, MoveTo(ui_x, 2), Print("║  ASTEROIDS    ║"))?;
    execute!(stdout, MoveTo(ui_x, 3), Print("╚═══════════════╝"))?;

    // My score
    let my_ship = state.ships.iter().find(|s| s.id == my_id);
    let my_score = my_ship.map(|s| s.score).unwrap_or(0);
    let alive = my_ship.map(|s| s.alive).unwrap_or(false);

    execute!(stdout, MoveTo(ui_x, 5), SetForegroundColor(Color::Green))?;
    execute!(stdout, Print(format!("{}", my_name)))?;
    execute!(stdout, MoveTo(ui_x, 6), SetForegroundColor(Color::White))?;
    execute!(stdout, Print(format!("Score: {}", my_score)))?;
    execute!(stdout, MoveTo(ui_x, 7))?;
    if alive {
        execute!(stdout, SetForegroundColor(Color::Green), Print("ALIVE"))?;
    } else {
        execute!(stdout, SetForegroundColor(Color::Red), Print("DEAD"))?;
    }

    // Leaderboard
    execute!(stdout, MoveTo(ui_x, 9), SetForegroundColor(Color::Yellow))?;
    execute!(stdout, Print("─ High Scores ─"))?;

    for (i, entry) in state.leaderboard.iter().take(5).enumerate() {
        execute!(stdout, MoveTo(ui_x, 10 + i as u16), ResetColor)?;
        let display = format!("{}. {} {}", i + 1, entry.name, entry.score);
        execute!(stdout, Print(&display[..display.len().min(15)]))?;
    }

    // Controls
    execute!(stdout, MoveTo(ui_x, 16), SetForegroundColor(Color::DarkGrey))?;
    execute!(stdout, Print("─ Controls ─"))?;
    execute!(stdout, MoveTo(ui_x, 17), Print("W/↑: Thrust"))?;
    execute!(stdout, MoveTo(ui_x, 18), Print("A/←: Left"))?;
    execute!(stdout, MoveTo(ui_x, 19), Print("D/→: Right"))?;
    execute!(stdout, MoveTo(ui_x, 20), Print("Space: Fire"))?;
    execute!(stdout, MoveTo(ui_x, 21), Print("Q: Quit"))?;

    // Status
    execute!(stdout, MoveTo(ui_x, 23), SetForegroundColor(Color::DarkGrey))?;
    execute!(stdout, Print(format!("Tick: {}", state.tick)))?;
    execute!(stdout, MoveTo(ui_x, 24))?;
    execute!(stdout, Print(format!("Players: {}", state.ships.len())))?;

    execute!(stdout, ResetColor)?;
    stdout.flush()?;
    Ok(())
}

fn get_ship_char(angle: f64) -> char {
    // Convert angle to 8-directional character
    let normalized = (angle + std::f64::consts::PI) % (2.0 * std::f64::consts::PI);
    let index = ((normalized / (2.0 * std::f64::consts::PI) * 8.0 + 0.5) as usize) % 8;

    match index {
        0 => '←',
        1 => '↙',
        2 => '↓',
        3 => '↘',
        4 => '→',
        5 => '↗',
        6 => '↑',
        7 => '↖',
        _ => '→',
    }
}
