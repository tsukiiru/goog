use std::io::{self, Write};
use std::time::Duration;

use rand::random_range;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::terminal_size;
use tokio::sync::oneshot;

const ESC_CODE: &str = "\x1b";
const CLEAR_CODE: &str = "[0m";

#[allow(dead_code)]
const FG_COLOR_CODE: [i8; 8] = [
    30, // black
    31, // red
    32, // green
    33, // yellow
    34, // blue
    35, // magenta
    36, // cyan
    37, // white
];

const GOOG: &str = include_str!("./goog-ascii-art.txt");
const SPEED: i16 = 1;
const WAIT: u64 = 50;

struct IVec2(i16, i16);
struct UVec2(u16, u16);

impl IVec2 {
    fn new(x: i16, y: i16) -> Self {
        IVec2(x, y)
    }
}

impl UVec2 {
    fn new(x: u16, y: u16) -> Self {
        UVec2(x, y)
    }
}

struct Goog {
    pos: UVec2,
    size: UVec2,
    velocity_mul: IVec2,
    speed: i16,
}

impl Goog {
    fn new() -> Self {
        Goog {
            pos: UVec2::new(1, 1),    // from top left corner
            size: UVec2::new(50, 24), // width, height in characters count of the ascii art
            velocity_mul: IVec2::new(1, 1),
            speed: SPEED,
        }
    }

    fn is_collide(&self, screen_size: &UVec2) -> Collision {
        let (mut h_collide, mut v_collide) = (None, None);

        if self.pos.0 <= 1 {
            h_collide = Some(HCollide::Left);
        } else if self.pos.0 + self.size.0 > screen_size.0 {
            h_collide = Some(HCollide::Right);
        }

        if self.pos.1 <= 1 {
            v_collide = Some(VCollide::Top);
        } else if self.pos.1 + self.size.1 > screen_size.1 {
            v_collide = Some(VCollide::Bottom);
        }

        (h_collide, v_collide)
    }

    fn apply_collision(&mut self, collision: &Collision) {
        if let Some(col) = &collision.0 {
            match col {
                HCollide::Left => self.velocity_mul.0 = 1,
                HCollide::Right => self.velocity_mul.0 = -1,
            }
        }

        if let Some(col) = &collision.1 {
            match col {
                VCollide::Top => self.velocity_mul.1 = 1,
                VCollide::Bottom => self.velocity_mul.1 = -1,
            }
        }
    }

    fn apply_velocity(&mut self) {
        self.pos.0 = (self.pos.0 as i16 + self.speed * self.velocity_mul.0) as u16;
        self.pos.1 = (self.pos.1 as i16 + self.speed * self.velocity_mul.1) as u16;
    }
}

type Collision = (Option<HCollide>, Option<VCollide>);
enum HCollide {
    Right,
    Left,
}
enum VCollide {
    Top,
    Bottom,
}

#[tokio::main]
async fn main() {
    let (tx, mut rx) = oneshot::channel::<bool>();

    tokio::spawn(async move {
        let stdin = io::stdin();
        for key in stdin.keys() {
            if key.unwrap() == Key::Esc {
                let _ = tx.send(true);
                break;
            }
        }
    });

    let mut stdout = io::stdout().into_raw_mode().unwrap();

    write!(
        stdout,
        "{}{}{}",
        termion::clear::All,
        termion::cursor::Hide,
        termion::cursor::Goto(1, 1)
    )
    .unwrap();
    stdout.flush().unwrap();

    let mut goog = Goog::new();
    let mut color = FG_COLOR_CODE[random_range(..FG_COLOR_CODE.len() - 1)];

    loop {
        if let Ok(e) = rx.try_recv()
            && e
        {
            break;
        }

        write!(stdout, "\x1b[?2026h").unwrap();
        stdout.flush().unwrap();

        for i in 0..(goog.size.1 + 2) {
            write!(
                stdout,
                "{}{}",
                termion::cursor::Goto(1, goog.pos.1 + i - 1),
                termion::clear::CurrentLine,
            )
            .unwrap();
        } // only clearing the using lines

        for (i, l) in GOOG.lines().enumerate() {
            write!(
                stdout,
                "{}",
                termion::cursor::Goto(goog.pos.0, goog.pos.1 + i as u16),
            )
            .unwrap();
            write!(
                stdout,
                "{}[{}m{}{}{}",
                ESC_CODE, color, l, ESC_CODE, CLEAR_CODE
            )
            .unwrap();
        }

        write!(stdout, "\x1b[?2026l").unwrap();
        stdout.flush().unwrap();

        let screen_size = terminal_size().unwrap();
        let screen_size = UVec2::new(screen_size.0, screen_size.1);
        let collision = goog.is_collide(&screen_size);

        if collision.0.is_some() || collision.1.is_some() {
            color = FG_COLOR_CODE[random_range(..FG_COLOR_CODE.len() - 1)];
        }

        goog.apply_collision(&collision);
        goog.apply_velocity();

        tokio::time::sleep(Duration::from_millis(WAIT)).await;
    }
}
