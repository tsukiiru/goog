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

const SPEED: i16 = 1;

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
    goog: &'static str,
    delay: u64,
    change_color: bool,
    disabled: bool,
}

const GOOG_TINY: &str = include_str!("../goog/goog-8");
const GOOG_SMALL: &str = include_str!("../goog/goog-16");
const GOOG_MEDIUM: &str = include_str!("../goog/goog-24");
const GOOG_BIG: &str = include_str!("../goog/goog-32");
const GOOG_HUGE: &str = include_str!("../goog/goog-32");

impl Goog {
    fn new(size: Option<&str>, delay: Option<u64>, change_color: bool, disabled: bool) -> Self {
        let mut goog = Goog {
            pos: UVec2::new(1, 1),  // from top left corner
            size: UVec2::new(0, 0), // width, height in characters count of the ascii art
            velocity_mul: IVec2::new(1, 1),
            speed: SPEED,
            goog: GOOG_MEDIUM,
            delay: 40,
            change_color,
            disabled,
        };

        if let Some(delay) = delay {
            goog.delay = delay;
        }

        if let Some(s) = size {
            goog.goog = match s {
                "tiny" => GOOG_TINY,
                "small" => GOOG_SMALL,
                "medium" => GOOG_MEDIUM,
                "big" => GOOG_BIG,
                "huge" => GOOG_HUGE,
                _ => GOOG_MEDIUM,
            };
        }

        let mut iter = goog.goog.lines();
        goog.size.0 = iter.nth(0).unwrap().chars().count() as u16;
        goog.size.1 = iter.count() as u16 + 1;

        goog
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

use clap::Parser;

/// Goog... (Esc to escape)
#[derive(Parser)]
#[command()]
struct Cli {
    /// delay between each "frames" (in milliseconds) (also makes escaping take longer)
    #[arg(short, long)]
    delay: Option<u64>,

    /// goog... size (tiny, small, medium, big, huge)
    #[arg(short, long)]
    size: Option<String>,

    /// whether to change color each bounce
    #[arg(short, long)]
    color: bool,

    /// remove his ability to move (...why would you do this. you monster.)
    #[arg(long)]
    disable: bool,
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

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

    let mut goog = Goog::new(args.size.as_deref(), args.delay, args.color, args.disable);
    let mut color = FG_COLOR_CODE[random_range(..FG_COLOR_CODE.len() - 1)];

    loop {
        if let Ok(e) = rx.try_recv()
            && e
        {
            write!(stdout, "{}", termion::cursor::Show).unwrap();
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

        for (i, l) in goog.goog.lines().enumerate() {
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

        if goog.change_color && (collision.0.is_some() || collision.1.is_some()) {
            color = FG_COLOR_CODE[random_range(..FG_COLOR_CODE.len() - 1)];
        }

        if !goog.disabled {
            goog.apply_collision(&collision);
            goog.apply_velocity();
        }

        tokio::time::sleep(Duration::from_millis(goog.delay)).await;
    }
}
