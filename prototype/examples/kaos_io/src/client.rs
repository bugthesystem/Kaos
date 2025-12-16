//! Kaos.io Terminal Client
//!
//! A terminal-based client for Kaos.io using crossterm for rendering.

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    style::{Color, Print, SetForegroundColor},
    terminal::{self, ClearType},
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::io::{stdout, Result, Write};
use std::time::{Duration, Instant};

const WORLD_WIDTH: f32 = 2000.0;
const WORLD_HEIGHT: f32 = 2000.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Player {
    id: String,
    name: String,
    x: f32,
    y: f32,
    mass: f32,
    color: String,
    score: i32,
    alive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Food {
    id: i32,
    x: f32,
    y: f32,
    radius: f32,
    color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LeaderboardEntry {
    id: String,
    name: String,
    score: i32,
}

struct GameState {
    my_id: String,
    /// Player name (stored for display)
    _my_name: String,
    players: Vec<Player>,
    food: Vec<Food>,
    leaderboard: Vec<LeaderboardEntry>,
    target_x: f32,
    target_y: f32,
    camera_x: f32,
    camera_y: f32,
    messages: Vec<String>,
}

impl GameState {
    fn new(name: &str) -> Self {
        let mut rng = rand::thread_rng();
        Self {
            my_id: "local".to_string(),
            _my_name: name.to_string(),
            players: vec![Player {
                id: "local".to_string(),
                name: name.to_string(),
                x: rng.gen_range(100.0..WORLD_WIDTH - 100.0),
                y: rng.gen_range(100.0..WORLD_HEIGHT - 100.0),
                mass: 400.0,  // Starting mass
                color: "#4ECDC4".to_string(),
                score: 0,
                alive: true,
            }],
            food: Self::spawn_food(200),
            leaderboard: vec![],
            target_x: WORLD_WIDTH / 2.0,
            target_y: WORLD_HEIGHT / 2.0,
            camera_x: WORLD_WIDTH / 2.0,
            camera_y: WORLD_HEIGHT / 2.0,
            messages: vec![
                "Welcome to Kaos.io!".to_string(),
                "Use WASD or arrow keys to move".to_string(),
                "Eat food and smaller players to grow".to_string(),
                "Press 'Q' to quit".to_string(),
            ],
        }
    }

    fn spawn_food(count: usize) -> Vec<Food> {
        let mut rng = rand::thread_rng();
        let colors = ["#FF6B6B", "#4ECDC4", "#45B7D1", "#96CEB4", "#FFEAA7", "#DDA0DD"];
        (0..count)
            .map(|i| Food {
                id: i as i32,
                x: rng.gen_range(50.0..WORLD_WIDTH - 50.0),
                y: rng.gen_range(50.0..WORLD_HEIGHT - 50.0),
                radius: 8.0,
                color: colors[rng.gen_range(0..colors.len())].to_string(),
            })
            .collect()
    }

    fn get_my_player(&self) -> Option<&Player> {
        self.players.iter().find(|p| p.id == self.my_id)
    }

    fn get_my_player_mut(&mut self) -> Option<&mut Player> {
        self.players.iter_mut().find(|p| p.id == self.my_id)
    }

    fn mass_to_radius(mass: f32) -> f32 {
        (mass / std::f32::consts::PI).sqrt() * 4.0
    }

    fn update(&mut self, dt: f32) {
        // Get target values
        let target_x = self.target_x;
        let target_y = self.target_y;

        // Update my player position
        let (player_x, player_y, player_radius, player_alive) = {
            if let Some(player) = self.players.iter_mut().find(|p| p.id == self.my_id) {
                if player.alive {
                    let dx = target_x - player.x;
                    let dy = target_y - player.y;
                    let dist = (dx * dx + dy * dy).sqrt();

                    let radius = Self::mass_to_radius(player.mass);
                    let speed = 200.0 * (20.0 / (radius * 0.5 + 10.0));

                    if dist > 5.0 {
                        player.x += (dx / dist) * speed * dt;
                        player.y += (dy / dist) * speed * dt;
                    }

                    // Clamp to world
                    player.x = player.x.clamp(radius, WORLD_WIDTH - radius);
                    player.y = player.y.clamp(radius, WORLD_HEIGHT - radius);

                    (player.x, player.y, radius, true)
                } else {
                    (0.0, 0.0, 0.0, false)
                }
            } else {
                (0.0, 0.0, 0.0, false)
            }
        };

        if !player_alive {
            return;
        }

        // Update camera
        self.camera_x = player_x;
        self.camera_y = player_y;

        // Check food collisions
        self.food.retain(|food| {
            let dist = ((player_x - food.x).powi(2) + (player_y - food.y).powi(2)).sqrt();
            dist >= player_radius - food.radius * 0.5
        });

        // Count eaten food and update player
        let eaten = 200 - self.food.len();
        if eaten > 0 {
            if let Some(player) = self.players.iter_mut().find(|p| p.id == self.my_id) {
                player.mass += eaten as f32 * 5.0;
                player.score += eaten as i32;
            }
        }

        // Respawn food
        while self.food.len() < 200 {
            let mut rng = rand::thread_rng();
            let colors = ["#FF6B6B", "#4ECDC4", "#45B7D1", "#96CEB4", "#FFEAA7"];
            self.food.push(Food {
                id: rng.gen(),
                x: rng.gen_range(50.0..WORLD_WIDTH - 50.0),
                y: rng.gen_range(50.0..WORLD_HEIGHT - 50.0),
                radius: 8.0,
                color: colors[rng.gen_range(0..colors.len())].to_string(),
            });
        }

        // Mass decay
        if let Some(player) = self.players.iter_mut().find(|p| p.id == self.my_id) {
            if player.mass > 800.0 {
                player.mass *= 1.0 - 0.001 * dt;
            }
        }

        // Update leaderboard
        self.leaderboard = self.players
            .iter()
            .filter(|p| p.alive)
            .map(|p| LeaderboardEntry {
                id: p.id.clone(),
                name: p.name.clone(),
                score: p.mass as i32,
            })
            .collect();
        self.leaderboard.sort_by(|a, b| b.score.cmp(&a.score));
    }
}

fn hex_to_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() >= 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
        Color::Rgb { r, g, b }
    } else {
        Color::White
    }
}

fn render(state: &GameState) -> Result<()> {
    let (term_width, term_height) = terminal::size()?;
    let mut stdout = stdout();

    // Clear screen
    execute!(stdout, terminal::Clear(ClearType::All))?;

    // Calculate viewport
    let view_width = term_width as f32 * 10.0;
    let view_height = term_height as f32 * 10.0;
    let view_left = state.camera_x - view_width / 2.0;
    let view_top = state.camera_y - view_height / 2.0;

    // World to screen conversion
    let to_screen = |wx: f32, wy: f32| -> (u16, u16) {
        let sx = ((wx - view_left) / view_width * term_width as f32) as u16;
        let sy = ((wy - view_top) / view_height * term_height as f32) as u16;
        (sx.min(term_width - 1), sy.min(term_height - 1))
    };

    // Draw food
    for food in &state.food {
        let (sx, sy) = to_screen(food.x, food.y);
        if sx < term_width && sy < term_height {
            execute!(
                stdout,
                cursor::MoveTo(sx, sy),
                SetForegroundColor(hex_to_color(&food.color)),
                Print("·")
            )?;
        }
    }

    // Draw players
    for player in &state.players {
        if !player.alive { continue; }

        let radius = GameState::mass_to_radius(player.mass);
        let screen_radius = (radius / view_width * term_width as f32).max(1.0) as i32;

        let (cx, cy) = to_screen(player.x, player.y);
        let color = hex_to_color(&player.color);

        // Draw player circle (simplified as a blob)
        for dy in -screen_radius..=screen_radius {
            for dx in -screen_radius..=screen_radius {
                if dx * dx + dy * dy <= screen_radius * screen_radius {
                    let sx = (cx as i32 + dx) as u16;
                    let sy = (cy as i32 + dy) as u16;
                    if sx < term_width && sy < term_height {
                        let ch = if dx == 0 && dy == 0 { '●' }
                            else if dx * dx + dy * dy < (screen_radius - 1).max(0).pow(2) { '█' }
                            else { '▓' };
                        execute!(
                            stdout,
                            cursor::MoveTo(sx, sy),
                            SetForegroundColor(color),
                            Print(ch)
                        )?;
                    }
                }
            }
        }

        // Draw player name
        if cx > 0 && cy > 0 {
            let name = &player.name;
            let name_x = (cx as i32 - name.len() as i32 / 2).max(0) as u16;
            execute!(
                stdout,
                cursor::MoveTo(name_x, cy.saturating_sub(screen_radius as u16 + 1)),
                SetForegroundColor(Color::White),
                Print(name)
            )?;
        }
    }

    // Draw HUD - Border
    execute!(
        stdout,
        SetForegroundColor(Color::DarkGrey),
    )?;

    // Top border
    for x in 0..term_width {
        execute!(stdout, cursor::MoveTo(x, 0), Print("─"))?;
    }
    // Bottom border
    for x in 0..term_width {
        execute!(stdout, cursor::MoveTo(x, term_height - 1), Print("─"))?;
    }

    // Draw leaderboard
    let lb_x = term_width - 25;
    execute!(
        stdout,
        cursor::MoveTo(lb_x, 1),
        SetForegroundColor(Color::Yellow),
        Print("🏆 Leaderboard")
    )?;

    for (i, entry) in state.leaderboard.iter().take(5).enumerate() {
        let is_me = entry.id == state.my_id;
        let color = if is_me { Color::Cyan } else { Color::White };
        let marker = if is_me { "→" } else { " " };
        execute!(
            stdout,
            cursor::MoveTo(lb_x, 2 + i as u16),
            SetForegroundColor(color),
            Print(format!("{} {}. {} ({})", marker, i + 1, &entry.name[..entry.name.len().min(10)], entry.score))
        )?;
    }

    // Draw my stats
    if let Some(player) = state.get_my_player() {
        execute!(
            stdout,
            cursor::MoveTo(2, 1),
            SetForegroundColor(Color::Cyan),
            Print(format!("Mass: {} | Score: {}", player.mass as i32, player.score))
        )?;
    }

    // Draw messages
    for (i, msg) in state.messages.iter().rev().take(3).enumerate() {
        execute!(
            stdout,
            cursor::MoveTo(2, term_height - 3 - i as u16),
            SetForegroundColor(Color::DarkGrey),
            Print(msg)
        )?;
    }

    // Draw controls hint
    execute!(
        stdout,
        cursor::MoveTo(2, term_height - 1),
        SetForegroundColor(Color::DarkGrey),
        Print("WASD/Arrows: Move | Space: Split | Q: Quit")
    )?;

    stdout.flush()?;
    Ok(())
}

fn main() -> Result<()> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    execute!(stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;

    // Get player name
    let name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| format!("Player{}", rand::thread_rng().gen_range(1000..9999)));

    let mut state = GameState::new(&name);
    let mut last_frame = Instant::now();

    println!("Starting Kaos.io client as '{}'...", name);

    loop {
        // Handle input
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if let Some(player) = state.get_my_player() {
                        let speed = 100.0;
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => break,
                            KeyCode::Char('w') | KeyCode::Up => {
                                state.target_y = (player.y - speed).max(0.0);
                            }
                            KeyCode::Char('s') | KeyCode::Down => {
                                state.target_y = (player.y + speed).min(WORLD_HEIGHT);
                            }
                            KeyCode::Char('a') | KeyCode::Left => {
                                state.target_x = (player.x - speed).max(0.0);
                            }
                            KeyCode::Char('d') | KeyCode::Right => {
                                state.target_x = (player.x + speed).min(WORLD_WIDTH);
                            }
                            KeyCode::Char(' ') => {
                                state.messages.push("Split! (Not implemented in offline mode)".to_string());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Update game
        let now = Instant::now();
        let dt = (now - last_frame).as_secs_f32();
        last_frame = now;
        state.update(dt);

        // Render
        render(&state)?;
    }

    // Cleanup
    execute!(stdout(), terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;

    println!("\nThanks for playing Kaos.io!");
    if let Some(player) = state.get_my_player() {
        println!("Final Score: {} | Mass: {}", player.score, player.mass as i32);
    }

    Ok(())
}
