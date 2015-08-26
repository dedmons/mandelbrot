#[macro_use]
extern crate clap;
extern crate time;
extern crate image;
extern crate threadpool;
extern crate rustc_serialize;

use clap::{Arg, App};
use time::PreciseTime;
use rustc_serialize::json;
use threadpool::ThreadPool;

use std::fs::File;
use std::path::Path;
use std::error::Error;
use std::io::prelude::*;
use std::sync::mpsc::channel;

#[derive(Debug, RustcDecodable, RustcEncodable)]
struct Size {
    width: f32,
    height: f32,
}

#[derive(Debug, RustcDecodable, RustcEncodable)]
struct Point {
    x: f32,
    y: f32,
}

#[derive(Debug, RustcDecodable, RustcEncodable)]
struct Rect {
    origin: Point,
    size: Size,
}

#[derive(Debug, RustcDecodable, RustcEncodable)]
struct Config {
    ppu: u32,
    limit: u32,
    color_steps: f32,
    color_components: u8,
    color_palette: Vec<Vec<f32>>,
    window: Rect,
}

fn idx2point(idx: u32, width: u32) -> Point {
    let x = idx % width;
    let y = idx / width;
    Point { x: x as f32, y: y as f32}
}

fn point2idx(p: Point, width: u32) -> u32 {
    width * (p.y as u32) + (p.x as u32)
}

fn mandelbrot(cx: f32, cy: f32, limit: u32) -> u32 {
    let mut x = cx;
    let mut y = cy;
    let mut count = 0;
    while count < limit {
        let xy = x * y;
        let xx = x * x;
        let yy = y * y;
        let sum = xx + yy;
        if sum > 4.0 {
            break
        }
        count += 1;
        x = xx - yy + cx;
        y = xy * 2.0 + cy;
    }
    count as u32
}

fn gen_mandelbrot(size: &Size, config: &Config) -> Vec<u32> {
    let window = &config.window;
    let limit = config.limit;
    
    let data_size = size.width as u32 * size.height as u32;
    let mut data = Vec::with_capacity(data_size as usize);
    
    for i in 0..data_size {
        let p = idx2point(i, size.width as u32);

        let px: f32 = p.x / size.width;
        let py: f32 = p.y / size.height;
        
        let cx = window.origin.x + px * window.size.width;
        let cy = (window.origin.y + window.size.height) - py * window.size.height;
        
        let c = mandelbrot(cx, cy, limit);
        
        data.push(c);
    }

    data
}

fn rbg_from_palette(palette: &Vec<Vec<f32>>, idx: usize) -> (f32, f32, f32) {
    let color = &palette[idx];
    (color[0], color[1], color[2])
}

fn color_for_val_with_config(val: u32, config: &Config) -> (u8, u8, u8) {
    let (r, g, b);
    
    let limit = config.limit;
    let steps = config.color_steps;
    let palette = &config.color_palette;
    
    if val == limit as u32 {
        r = 0;
        g = 0;
        b = 0;
    } else {
        let val = (val as f32 % steps) * (palette.len() as f32) / steps;
        let left = val as usize % palette.len();
        let right = (left + 1) % palette.len();

        let p = val - left as f32;
        let (r1, g1, b1) = rbg_from_palette(palette, left);
        let (r2, g2, b2) = rbg_from_palette(palette, right);
        r = (r1 + (r2 - r1) * p) as u8;
        g = (g1 + (g2 - g1) * p) as u8;
        b = (b1 + (b2 - b1) * p) as u8;
    }
    (r, g, b)
}

fn validate_config(conf: &Config) {

    // Check if limit is 'to large'
    if conf.limit > 10000 {
        panic!("Config Error: limit is over 10,000");
    }

    // Check if color component count is valid
    match conf.color_components {
        3 => println!("Using RBG colors"),
        _ => panic!("Unsuported color component count"),
    };
    
    // Check colors in palette match component count
    for v in &conf.color_palette {
        if v.len() != conf.color_components as usize {
            panic!("Config Error: Color {:?} does not match color component count", v);
        }
    }
}

fn main() {
    let start = PreciseTime::now();
    
    let args = App::new("Mandelbrot Generator")
        .version(&crate_version!()[..])
        .author("DJ Edmonson <djedmonson@gmail.com>")
        .about("Generates a mandelbrot image")
        .arg(Arg::with_name("CONFIG")
             .long("config")
             .help("Config JSON file to use. Output will be at <input_file_path>.png")
             .required(true)
             .takes_value(true))
        .arg(Arg::with_name("output-palette")
             .long("output-palette")
             .help("Generate image with 100px squares of the provided colors in order. Outputs to <input_file_path>-palette.png"))
        .get_matches();

    let config_file_path = Path::new(args.value_of("CONFIG").unwrap());
    println!("Getting config from {}", config_file_path.display());
    
    let mut config_file = match File::open(&config_file_path) {
        Err(why) => panic!("Could not open config file at {}: {}",
                           config_file_path.display(),
                           Error::description(&why)),
        Ok(f) => f,
    };

    let mut config_json = String::new();
    match config_file.read_to_string(&mut config_json) {
        Err(why) => panic!("Could not read config file at {}: {}",
                           config_file_path.display(),
                           Error::description(&why)),
        Ok(_) => println!("Read config file"),
    };

    let config: Config = match json::decode(&config_json) {
        Err(why) => panic!("Error parsing config JSON: {}",
                           why),
        Ok(conf) => conf,
    };

    validate_config(&config);

    println!("Bootstrap time:\n{}", start.to(PreciseTime::now()));
    
    if args.is_present("output-palette") {
        let root = match config_file_path.file_stem() {
            None => unreachable!(),
            Some(r) => r.to_str().unwrap().to_string(),
        };

        let output_path_string = root + "-palette.png";
        let output_path = Path::new(&output_path_string);
        println!("Generating color palette image at {}", output_path.display());

        let width = config.color_palette.len() * 100;
        let height = 100;

        let mut imgbuf = image::ImageBuffer::new(width as u32, height as u32);

        for (x, _, pixel) in imgbuf.enumerate_pixels_mut() {
            let color_idx = (x as f32 / width as f32) * config.color_palette.len() as f32;

            let (r, b, g) = rbg_from_palette(&config.color_palette, color_idx as usize);

            *pixel = image::Rgb([r as u8, b as u8, g as u8]);
        }

        let _ = imgbuf.save(output_path);
    } else {
        let output_path = config_file_path.with_extension("png");

        let img_width = config.ppu as f32 * config.window.size.width;
        let img_height = config.ppu as f32 * config.window.size.height;

        println!("Generating image at {} with size {}x{}", output_path.display(), img_width, img_height);
        
        let size = Size {width: img_width, height: img_height};
        
        let mut imgbuf = image::ImageBuffer::new(size.width as u32, size.height as u32);

        let render_start = PreciseTime::now();
        let mut phase_start = PreciseTime::now();
        
        let imgdata = gen_mandelbrot(&size, &config);

        println!("Generation Duration:\n{}", phase_start.to(PreciseTime::now()));

        phase_start = PreciseTime::now();
        
        for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
            let idx = point2idx(Point{ x: x as f32, y: y as f32}, size.width as u32) as usize;
        
            let it = imgdata[idx];

            let (r, g, b) = color_for_val_with_config(it, &config);
            
            *pixel = image::Rgb([r as u8, g as u8, b as u8]);
        }
        println!("Render Duration:\n{}", phase_start.to(PreciseTime::now()));
        phase_start = PreciseTime::now();

        let _ = imgbuf.save(output_path);
        println!("Save Duration:\n{}", phase_start.to(PreciseTime::now()));
        println!("Total Render Duration:\n{}", render_start.to(PreciseTime::now()));
    }

    println!("Total time:\n{}", start.to(PreciseTime::now()));
}

