use std::{error::Error, fmt::Display, fs::OpenOptions, io::BufWriter, path::PathBuf};

use clap::Parser;
use image::{io::Reader as ImageReader, GenericImageView};
use serde::Serialize;
use tracing::debug;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SquareType {
    Wall,
    Checkpoint,
    Start,
    End,
    Empty,
}

impl Display for SquareType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

impl From<[u8; 4]> for SquareType {
    fn from(value: [u8; 4]) -> Self {
        match value {
            [0, 0, 0, _] => Self::Wall,         // Black
            [255, 0, 0, _] => Self::End,        // Red
            [0, 255, 0, _] => Self::Start,      // Green
            [0, 0, 255, _] => Self::Checkpoint, // Blue
            [255, 255, 255, _] => Self::Empty,  // White
            _ => unimplemented!("{:?}", value),
        }
    }
}

/// Lvl maker from image
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    image: PathBuf,

    #[clap(short, long)]
    outfile: Option<PathBuf>,

    #[clap(short, long, default_value = "false")]
    pretty: bool,
}

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
struct Point {
    x: u32,
    y: u32,
}

impl Point {
    fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
struct Wall {
    start: Point,
    end: Option<Point>,
}

impl Wall {
    fn length(self) -> u32 {
        if let Some(end) = self.end {
            (end.x - self.start.x) + (end.y - self.start.y)
        } else {
            1
        }
    }
}

impl Ord for Wall {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_length = self.length();
        let other_length = other.length();

        self_length.cmp(&other_length)
    }
}

impl PartialOrd for Wall {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Serialize)]
struct Lvl {
    width: u32,
    height: u32,
    walls: Vec<Wall>,
    start: Point,
    end: Point,
    checkpoints: Vec<Point>,
}

#[inline]
fn check_if_point_is_wall(x: u32, y: u32, walls: &[Wall]) -> bool {
    walls.iter().any(|wall| {
        (wall.start.x <= x && x <= wall.end.map_or_else(|| wall.start.x, |end| end.x))
            && (wall.start.y <= y && y <= wall.end.map_or_else(|| wall.start.y, |end| end.y))
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let img = ImageReader::open(args.image)?.decode()?;
    debug!("Lvl Size {}x{}", img.width(), img.height());
    let mut lvl = Lvl {
        width: img.width(),
        height: img.height(),
        walls: Vec::new(),
        start: Point { x: 0, y: 0 },
        end: Point { x: 0, y: 0 },
        checkpoints: Vec::new(),
    };

    let mut horizontal_walls = Vec::new();

    let mut x;
    let mut y = 0;
    while y < img.height() {
        x = 0;
        while x < img.width() {
            let pixel = img.get_pixel(x, y);
            match SquareType::from(pixel.0) {
                SquareType::Wall => {
                    // Only check for horizontal lines
                    let start = Point::new(x, y);
                    while (x + 1) < img.width()
                        && SquareType::from(img.get_pixel(x + 1, y).0) == SquareType::Wall
                    {
                        x += 1;
                        tracing::trace!("Wall detected at: {}-{}", x, y);
                    }

                    // Always insert, even if it's a single wall block
                    horizontal_walls.push(Wall {
                        start,
                        end: (start.x != x).then_some(Point::new(x, y)),
                    })
                }
                SquareType::End => lvl.end = Point::new(x, y),
                SquareType::Checkpoint => lvl.checkpoints.push(Point::new(x, y)),
                SquareType::Start => lvl.start = Point::new(x, y),
                SquareType::Empty => (),
            }

            x += 1;
        }
        y += 1;
    }

    // Add vertical walls
    let mut vertical_walls = Vec::new();
    x = 0;
    while x < img.width() {
        y = 0;
        while y < img.height() {
            let pixel = img.get_pixel(x, y);
            if SquareType::from(pixel.0) == SquareType::Wall {
                let start = Point::new(x, y);

                while (y + 1) < img.height()
                    && SquareType::from(img.get_pixel(x, y + 1).0) == SquareType::Wall
                {
                    y += 1;
                    tracing::trace!("Wall detected at: {}-{}", x, y);
                }

                let wall = Wall {
                    start,
                    end: (start.y != y).then_some(Point::new(x, y)),
                };

                // Only insert none 1 block walls
                if wall.end.is_some() {
                    debug!("{:?}", wall);
                    vertical_walls.push(wall);
                }
            }

            y += 1
        }

        x += 1;
    }

    // Filter single block horizontal_walls that are in multi block vertical walls
    let mut walls: Vec<Wall> = horizontal_walls
        .into_iter()
        .filter(|h_wall| {
            h_wall.end.is_some()
                || !check_if_point_is_wall(h_wall.start.x, h_wall.start.y, &vertical_walls)
        })
        .collect();

    walls.append(&mut vertical_walls);

    walls.sort();
    walls.reverse();

    lvl.walls = walls;

    if let Some(outfile) = args.outfile {
        let handle = OpenOptions::new().write(true).create(true).open(outfile)?;
        let writer = BufWriter::new(handle);
        if args.pretty {
            serde_json::to_writer_pretty(writer, &lvl)?;
        } else {
            serde_json::to_writer(writer, &lvl)?;
        }
    } else {
        let handle = std::io::stdout();
        let writer = BufWriter::new(handle);
        if args.pretty {
            serde_json::to_writer_pretty(writer, &lvl)?;
        } else {
            serde_json::to_writer(writer, &lvl)?;
        }
    };

    Ok(())
}
