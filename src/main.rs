#[macro_use]
extern crate clap;
use clap::{Arg, App};

extern crate image;

use std::path::Path;

#[derive(Debug)]
struct Size {
    width: f32,
    height: f32,
}

#[derive(Debug)]
struct Point {
    x: f32,
    y: f32,
}

#[derive(Debug)]
struct Rect {
    origin: Point,
    size: Size,
}

const MAXITER: usize = 1000;

const COLORS: &'static [(f32, f32, f32)] = &[(0.0, 7.0, 100.0),
                                             (32.0, 107.0, 203.0),
                                             (237.0, 255.0, 255.0),
                                             (255.0, 170.0, 0.0),
                                             (0.0, 2.0, 0.0)];
const SCALE: f32 = 50.0;


fn size_parser(input: String) -> Option<Size> {
    let v: Vec<&str> = input.split('x').collect();
    
    if v.len() == 2 {
        let w = v[0].parse::<u32>();
        let h = v[1].parse::<u32>();
        
        if w.is_ok() && h.is_ok() {
            Some( Size { width: w.unwrap() as f32, height: h.unwrap() as f32 } )
        } else {
            None
        }
    } else {
        None
    }
}

fn idx2point(idx: u32, width: u32) -> Point {
    let x = idx % width;
    let y = idx / width;
    Point { x: x as f32, y: y as f32}
}

fn point2idx(p: Point, width: u32) -> u32 {
    width * (p.y as u32) + (p.x as u32)
}

fn mandelbrot(cx: f32, cy: f32) -> u32 {
    let mut x = cx;
    let mut y = cy;
    let mut count = 0;
    while count < MAXITER {
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

fn gen_mandelbrot(size: &Size, window: &Rect) -> Vec<u32> {
    let data_size = size.width as u32 * size.height as u32;
    let mut data = Vec::with_capacity(data_size as usize);
    
    for i in 0..data_size {
        let p = idx2point(i, size.width as u32);

        let px: f32 = p.x / size.width;
        let py: f32 = p.y / size.height;
        
        let cx = window.origin.x + px * window.size.width;
        let cy = (window.origin.y + window.size.height) - py * window.size.height;
        
        let c = mandelbrot(cx, cy);
        
        data.push(c);
    }

    data
}

fn color_for_val(val: u32) -> (u8, u8, u8) {
    let (r, g, b);
    if val == MAXITER as u32 {
        r = 0;
        g = 0;
        b = 0;
    } else {
        let val = (val as f32 % SCALE) * (COLORS.len() as f32) / SCALE;
        let left = val as usize % COLORS.len();
        let right = (left + 1) % COLORS.len();

        let p = val - left as f32;
        let (r1, g1, b1) = COLORS[left];
        let (r2, g2, b2) = COLORS[right];
        r = (r1 + (r2 - r1) * p) as u8;
        g = (g1 + (g2 - g1) * p) as u8;
        b = (b1 + (b2 - b1) * p) as u8;
    }
    (r as u8, g as u8, b as u8)
}

fn main() {    
    let args = App::new("Mandelbrot Generator")
        .version(&crate_version!()[..])
        .author("DJ Edmonson <djedmonson@gmail.com>")
        .about("Generates a mandelbrot image")
        .arg(Arg::with_name("SIZE")
             .short("s")
             .long("size")
             .help("Size of image to produce in pixels")
             .required(true)
             .takes_value(true)
             .validator(|input| {
                 let res = size_parser(input);
                 if res.is_some() {
                     Ok(())
                 } else {
                     Err(String::from("Size must be specified in the format (width)x(height), where width and height are integers"))
                 }
             }))
        .arg(Arg::with_name("FILE")
             .short("f")
             .long("file")
             .help("File path to save image to")
             .required(true)
             .takes_value(true))
        .arg(Arg::with_name("MIN-X")
             .long("min-x")
             .help("Minimum x value of set. Defaults to -2.0")
             .takes_value(true))
        .arg(Arg::with_name("MAX-X")
             .long("max-x")
             .help("Maximum x value of set. Defaults to 2.0")
             .takes_value(true))
        .arg(Arg::with_name("MIN-Y")
             .long("min-y")
             .help("Minimum y value of set. Defaults to -2.0")
             .takes_value(true))
        .arg(Arg::with_name("MAX-Y")
             .long("max-y")
             .help("Maximum y value of set. Defaults to 2.0")
             .takes_value(true))
        .get_matches();

    let size = size_parser(args.value_of("SIZE").unwrap().to_string()).unwrap();
    println!("Value for size: {:?}", size);

    let file_path = args.value_of("FILE").unwrap();
    println!("Output File Path: {:?}", file_path);

    let min_x = args.value_of("MIN-X").unwrap_or("-2.0").parse::<f32>().unwrap_or_else(|e| { panic!("min_x err:  {}", e) });
    let max_x = args.value_of("MAX-X").unwrap_or("2.0").parse::<f32>().unwrap_or_else(|e| { panic!("max_x err:  {}", e) });
    let min_y = args.value_of("MIN-Y").unwrap_or("-2.0").parse::<f32>().unwrap_or_else(|e| { panic!("min_y err:  {}", e) });
    let max_y = args.value_of("MAX-Y").unwrap_or("2.0").parse::<f32>().unwrap_or_else(|e| { panic!("max_y err:  {}", e) });
    
    let width = max_x - min_x;
    let height = max_y - min_y;
    
    let window = Rect { origin: Point {x: min_x, y: min_y}, size: Size {width: width, height: height} };
    println!("Output window: {:?}", window);
    
    let mut imgbuf = image::ImageBuffer::new(size.width as u32, size.height as u32);

    let imgdata = gen_mandelbrot(&size, &window);
    
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let idx = point2idx(Point{ x: x as f32, y: y as f32}, size.width as u32) as usize;
        
        let it = imgdata[idx];

        let (r, g, b) = color_for_val(it);
        
        *pixel = image::Rgb([r, g, b]);
    }

    let _ = imgbuf.save(Path::new(file_path));
}
