extern crate sdl2;
extern crate spmc;

use std::thread;
use std::time::Instant;
use std::ops::{ Add, Sub, Mul };
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Point;

const WIDTH: i32 = 512;
const HEIGHT: i32 = 512;
const NUM_THREADS: usize = 8;
const BLOCK_WIDTH: i32 = 16;
const BLOCK_HEIGHT: i32 = 16;

// 4.28 fixpoint number type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Fix28(i32);

impl Add for Fix28 {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Fix28(self.0 + other.0)
    }
}

impl Sub for Fix28 {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Fix28(self.0 - other.0)
    }
}

impl Mul for Fix28 {
    type Output = Self;

    fn mul(self, other: Self) -> Self::Output {
        Fix28(((self.0 as i64 * other.0 as i64) >> 28) as i32)
    }
}

impl From<f32> for Fix28 {
    fn from(value: f32) -> Self {
        Fix28((value * (1 << 28) as f32).round() as i32)
    }
}

impl From<Fix28> for f32 {
    fn from(value: Fix28) -> Self {
        value.0 as f32 / (1 << 28) as f32
    }
}

fn is_mandelbrot_member(c_re: Fix28, c_im: Fix28) -> bool {
    let zero = Fix28::from(0.0);
    let two  = Fix28::from(2.0);
    let four = Fix28::from(4.0);

    // Fast bail-out path: the biggest cardioid and the somewhat
    // smaller circle centered at -1 can be easily detected;
    // they save us approximately half of the work.
    let small_circle_center_x = Fix28::from(-1.0);
    let small_circle_center_y = zero;
    let small_circle_radius_sq = Fix28::from(0.25 * 0.25);
    let big_circle_center_x = Fix28::from(-0.265625);
    let big_circle_center_y = zero;
    let big_circle_radius_sq = Fix28::from(0.5 * 0.5);

    let dx_small = c_re - small_circle_center_x;
    let dy_small = c_im - small_circle_center_y;
    let dx_big = c_re - big_circle_center_x;
    let dy_big = c_im - big_circle_center_y;

    if dx_big * dx_big + dy_big * dy_big < big_circle_radius_sq {
        return true
    }

    if dx_small * dx_small + dy_small * dy_small < small_circle_radius_sq {
        return true
    }

    // Outside the circles, perform the slow iteration.
    let mut z_re = zero;
    let mut z_im = zero;

    for _ in 0..1000 {
        if z_re * z_re + z_im * z_im > four {
            return false
        }

        let z_re_next = z_re * z_re - z_im * z_im + c_re;
        z_im = two * z_re * z_im + c_im;
        z_re = z_re_next;
    }

    true
}


fn compute_block(x: i32, y: i32, points: &mut Vec<Point>) {
    for x in x..x + BLOCK_WIDTH {
        for y in y..y + BLOCK_HEIGHT {
            // Convert coordinates to 4.28 fixpoint
            let re = Fix28((x - 3 * (WIDTH / 4)) << 20);
            let im = Fix28((y - (HEIGHT / 2)) << 20);

            if is_mandelbrot_member(re, im) {
                // symmetry along the Y axis: z in M <=> z* in M
                points.push(Point::new(x, y));
                points.push(Point::new(x, HEIGHT - y));
            }
        }
    }
}

fn main() {
    // Initialize SDL-based graphics
    let ctx = sdl2::init().unwrap();
    let mut pump = ctx.event_pump().unwrap();
    let video = ctx.video().unwrap();
    let window = video.window("Mandelbrot", WIDTH as u32, HEIGHT as u32).build().unwrap();
    let mut canvas = window.into_canvas().build().unwrap();

    let t0 = Instant::now();

    // Compute in the range:
    // [-1.5...+0.5] x [-1.0...+1.0]:
    //   x in [0..WIDTH)
    //   y in [HEIGHT / 2..HEIGHT) // symmetry along the Y axis

    let (tx, rx) = spmc::channel();

    // Break up the canvas into blocks. Write all blocks
    // into the channel. Thread workers will pick the next
    // available block on a first come -- first served basis.
    // This ensures approximately equal distribution of work.
    for x in 0..WIDTH / BLOCK_WIDTH {
        for y in HEIGHT / 2 / BLOCK_HEIGHT..HEIGHT / BLOCK_HEIGHT {
            tx.send((x * BLOCK_WIDTH, y * BLOCK_HEIGHT)).unwrap();
        }
    }

    let handles: Vec<_> = (0..NUM_THREADS).map(|_| {
        let rx = rx.clone();

        thread::spawn(move || {
            let mut points = Vec::new();

            while let Ok((x, y)) = rx.try_recv() {
                compute_block(x, y, &mut points);
            }

            points
        })
    }).collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // When all threads finished, draw the pixels.
    canvas.set_draw_color(Color::RGB(0xff, 0xff, 0xff));

    for points in results {
        canvas.draw_points(&points[..]).unwrap();
    }

    canvas.present();

    let t1 = Instant::now();
    let dt = t1 - t0;
    let secs = dt.as_secs() as f32 + dt.subsec_nanos() as f32 * 1e-9;

    println!("{:.3} s ({:.0} FPS)", secs, 1.0 / secs);

    // Wait for Escape keypress or the user closing the window
    loop {
        for event in pump.poll_iter() {
            if let Event::KeyUp { keycode: Some(Keycode::Escape), .. } = event {
                return
            }

            if let Event::Quit { .. } = event {
                return
            }
        }
    }
}
