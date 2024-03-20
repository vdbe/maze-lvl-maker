use std::{error::Error, fmt::Display, fs::OpenOptions, io::BufWriter, path::PathBuf};

use clap::Parser;
use image::{io::Reader as ImageReader, DynamicImage, GenericImageView};
use serde::Serialize;
use serde_json::{
    ser::{CompactFormatter, Formatter, PrettyFormatter},
    Serializer,
};
use tracing::{info, instrument};
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

#[derive(Serialize)]
struct Lvl {
    width: u32,
    height: u32,
    walls: Vec<Wall>,
    start: Point,
    end: Point,
    checkpoints: Vec<Point>,
}

#[instrument(skip(img, walls), ret)]
fn insert_walls(x: &mut u32, mut y: u32, img: &DynamicImage, walls: &mut Vec<Wall>) {
    let start = Point::new(*x, y);

    // Check horizontal
    while (*x + 1) < img.width() && SquareType::from(img.get_pixel(*x + 1, y).0) == SquareType::Wall
    {
        *x += 1;
        tracing::info!("Wall detected at: {}-{}", x, y);
    }
    let x_wall = Wall {
        start,
        end: (start.x != *x).then_some(Point::new(*x, y)),
    };

    // Check Vertical
    while (y + 1 < img.height())
        && SquareType::from(img.get_pixel(start.x, y + 1).0) == SquareType::Wall
    {
        y += 1;
        tracing::info!("Wall detected at: {}-{}", x, y);
    }
    let y_wall = Wall {
        start,
        end: (start.y != y).then_some(Point::new(*x, y)),
    };

    let already_in_list = walls.iter().any(|wall| {
        let same_column = wall.start.x == start.x;
        let y_start_larger = start.y >= wall.start.y;
        let y_end_smaller = y_wall.end.is_none()
            || y_wall
                .end
                .is_some_and(|end| wall.end.is_some_and(|w_end| end.y <= w_end.y));
        same_column && y_start_larger && y_end_smaller
    });

    if !already_in_list {
        if x_wall == y_wall {
            walls.push(x_wall);
        } else {
            if x_wall.end.is_some() {
                walls.push(x_wall);
            }
            if y_wall.end.is_some() {
                walls.push(y_wall);
            }
        }
    };
}

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let img = ImageReader::open(args.image)?.decode()?;
    info!("Lvl Size {}x{}", img.width(), img.height());
    let mut lvl = Lvl {
        width: img.width(),
        height: img.height(),
        walls: Vec::new(),
        start: Point { x: 0, y: 0 },
        end: Point { x: 0, y: 0 },
        checkpoints: Vec::new(),
    };

    let mut x;
    let mut y = 0;
    while y < img.height() {
        x = 0;
        while x < img.width() {
            let pixel = img.get_pixel(x, y);
            match SquareType::from(pixel.0) {
                SquareType::Wall => {
                    insert_walls(&mut x, y, &img, &mut lvl.walls);
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
