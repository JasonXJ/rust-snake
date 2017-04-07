extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
extern crate rand;

use std::collections::VecDeque;
use std::ops::{ Mul, Add, Sub };
use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use piston::input::keyboard::Key;
use glutin_window::GlutinWindow as Window;
use opengl_graphics::{ GlGraphics, OpenGL };
use graphics::types::Color;

const BLOCK_SIZE: usize = 10;
const GRID_WIDTH: usize = 90;
const GRID_HEIGHT: usize = 75;
const UPDATE_PER_SECONDS: u64 = 30;
const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const SNAKE_INIT_LEN: usize = 10;

struct App {
    gl: GlGraphics,  // OpenGL drawing backend.
    grid: Grid,
    repaint_all: bool,
    snake: Snake,
    food: Coordinate,
    gameover: bool,
}

#[derive(Eq, PartialEq)]
enum Fate {
    Die,
    Eat,
    Move,
}

// TODO: implement restart
impl App {
    fn new(gl: GlGraphics) -> App {
        let mut app = App {
            gl: gl,
            grid: Grid::new(GRID_WIDTH, GRID_HEIGHT, BLOCK_SIZE, WHITE),
            repaint_all: true,
            snake: Snake::new(((SNAKE_INIT_LEN-1) as isize, (GRID_HEIGHT/2) as isize), SNAKE_INIT_LEN),
            food: Coordinate{ x: 0, y: 0 },
            gameover: false,
        };

        // Initialize the grid with the snake
        for c in &app.snake.body {
            app.grid.update(*c, BLACK);
        }

        app.renew_food();

        app
    }

    fn render(&mut self, args: &RenderArgs) {
        self.grid.render(&mut self.gl, args, self.repaint_all);
        self.repaint_all = false;
    }

    fn update(&mut self, _: &UpdateArgs) {
        if !self.gameover {
            match self.determine_fate() {
                Fate::Die => { self.gameover = true; }
                f => {
                    let (new_head, optional_tail) = self.snake.update(f == Fate::Eat);
                    self.grid.update(new_head, BLACK);
                    if let Some(tail) = optional_tail {
                        self.grid.update(tail, WHITE);
                    }
                    if f == Fate::Eat {
                        self.renew_food();
                    }
                }
            }
        }
    }

    fn determine_fate(&self) -> Fate {
        let c = self.snake.next_head();
        if c.x >= 0 && c.x < GRID_WIDTH as isize && c.y >=0 && c.y < GRID_HEIGHT as isize {
            if self.grid.color_grid[c.y as usize][c.x as usize] == WHITE {
                return Fate::Move;
            } else if c == self.food {
                return Fate::Eat
            } 
        }
        Fate::Die
    }

    fn button_pressed(&mut self, button: &Button) {
        if !self.gameover {
            if let &Button::Keyboard(key) = button {
                if let Some(d) = Direction::from_key(key) {
                    self.snake.try_redirect(d);
                }
            }
        }
    }

    /// Randomly generate a new coordinate for self.food, and update self.grid with the new food.
    /// Note that this function does not care of the old food.
    fn renew_food(&mut self) {
        loop {
            self.food = Coordinate {
                x: (rand::random::<usize>() % GRID_WIDTH) as isize,
                y: (rand::random::<usize>() % GRID_HEIGHT) as isize
            };
            if self.grid.color_grid[self.food.y as usize][self.food.x as usize] != BLACK {
                break
            }
        }

        self.grid.update(self.food, BLACK);
    }
}

enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn to_relative_coordinate(&self) -> Coordinate {
        let pair = match *self {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        };
        Coordinate::from(pair)
    }

    fn from_key(key: Key) -> Option<Direction> {
        match key {
            Key::Up => Some(Direction::Up),
            Key::Down => Some(Direction::Down),
            Key::Left => Some(Direction::Left),
            Key::Right => Some(Direction::Right),
            _ => None,
        }
    }
}

impl Mul<Coordinate> for isize {
    type Output = Coordinate;

    fn mul(self, rhs: Coordinate) -> Coordinate {
        Coordinate { x: self * rhs.x, y: self * rhs.y}
    }
}

impl Add for Coordinate {
    type Output = Coordinate;
    
    fn add(self, rhs: Coordinate) -> Coordinate {
        Coordinate { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

impl Sub for Coordinate {
    type Output = Coordinate;
    
    fn sub(self, rhs: Coordinate) -> Coordinate {
        Coordinate { x: self.x - rhs.x, y: self.y - rhs.y }
    }
}

struct Snake {
    body: VecDeque<Coordinate>,
    direction: Direction,
}

impl Snake {
    fn new<C: Into<Coordinate>>(head: C, len: usize) -> Snake {
        let head = head.into();
        let mut snake = Snake {
            body: VecDeque::new(),
            direction: Direction::Right,
        };
        for x in (head.x-(len as isize)+1)..(head.x+1) {
            snake.body.push_front(Coordinate { x: x, y: head.y })
        }
        snake
    }

    /// Move the snake 1 step forward. Note that caller should make sure that the move is legal.
    /// Return value is a tuple. The first element is the coordination of the new head. The second
    /// is the coordination of the old tail, which might be None.
    fn update(&mut self, growed: bool) -> (Coordinate, Option<Coordinate>) {
        // XXX: Compiler complains about `self.body.push_front(self.next_head())`, but I think that
        // should be considered safe.
        let next_head = self.next_head();
        self.body.push_front(next_head);
        let mut old_tail = None;
        if !growed {
            old_tail = self.body.pop_back();
        }
        (next_head, old_tail)
    }

    fn next_head(&self) -> Coordinate {
        match self.body.front() {
            Some(&c) => c + self.direction.to_relative_coordinate(),
            None => panic!("Snake has no body!"),
        }
    }
    
    /// Try to redirect the snake. Note that a snake cannot be redirect backward.
    fn try_redirect(&mut self, direction: Direction) {
        if self.body[1] - self.body[0] != direction.to_relative_coordinate() {
            self.direction = direction;
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct Coordinate {
    x: isize,
    y: isize,
}

impl From<(isize, isize)> for Coordinate {
    fn from(pair: (isize, isize)) -> Coordinate {
        Coordinate{ x: pair.0, y: pair.1 }
    }
}

struct Grid {
    color_grid: Vec<Vec<Color>>,
    block_size: usize,  // This is for drawing
    pending: Vec<Coordinate>,
}

impl Grid {
    /// Construct a new `Grid`
    fn new(width: usize, height: usize, block_size: usize, init_color: Color) -> Grid {
        Grid {
            color_grid: vec![vec![init_color; width]; height],
            block_size: block_size,
            pending: Vec::new(),
        }
    }

    fn update<C>(&mut self, coordinate: C, color: Color) where C: Into<Coordinate> {
        let coordinate = coordinate.into();
        self.color_grid[coordinate.y as usize][coordinate.x as usize] = color;
        self.pending.push(coordinate);
    }

    fn render_block<C>(&self, gl: &mut GlGraphics, render_args: &RenderArgs, coordinate: C) where C: Into<Coordinate> {
        use graphics::rectangle;

        let coordinate = coordinate.into();
        let square = rectangle::square(
            (self.block_size as isize * coordinate.x) as f64,
            (self.block_size as isize * coordinate.y) as f64,
            self.block_size as f64);
        gl.draw(render_args.viewport(), |c, gl| {
            rectangle(self.color_grid[coordinate.y as usize][coordinate.x as usize], square, c.transform, gl);
        });
    }

    fn render(&mut self, gl: &mut GlGraphics, render_args: &RenderArgs, full: bool) {
        if self.color_grid.is_empty() || self.color_grid[0].is_empty() {
            return;
        }
        match full {
            true => {
                // TODO: This seems not very efficient because there will be bound check every time
                // in the "render_block"
                for y in 0..self.color_grid.len() {
                    for x in 0..self.color_grid[0].len() {
                        self.render_block(gl, render_args, (x as isize, y as isize));
                    }
                }
            },
            false => {
                // In this case, only render those in `self.pending`
                for coordinate in &self.pending {
                    self.render_block(gl, render_args, *coordinate);
                }
            },
        }
        self.pending.clear();
    }
}

fn main() {
    let opengl = OpenGL::V3_2;

    let mut window: Window = WindowSettings::new(
        "Snake", [(BLOCK_SIZE * GRID_WIDTH) as u32 , (BLOCK_SIZE * GRID_HEIGHT) as u32]
        )
        .opengl(opengl)
        .exit_on_esc(true)
        .build()
        .unwrap();

    let mut app = App::new(GlGraphics::new(opengl));

    let mut event_settings = EventSettings::new();
    event_settings.ups = UPDATE_PER_SECONDS;
    let mut events = Events::new(event_settings);
    while let Some(e) = events.next(&mut window) {
        match e {
            Input::Render(args) => app.render(&args),
            Input::Update(args) => app.update(&args),
            Input::Press(button) => app.button_pressed(&button),
            _ => {},
        }
    }
}
