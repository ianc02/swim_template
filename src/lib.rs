#![no_std]
#![feature(prelude_2024)]

//use alloc::string::String;
// use file_system_solution::{FileSystem, FileSystemResult};
use pc_keyboard::{DecodedKey, KeyCode};
use pluggable_interrupt_os::vga_buffer::{BUFFER_WIDTH, BUFFER_HEIGHT, plot, ColorCode, Color, plot_str, is_drawable, plot_num};
use ramdisk::RamDisk;
//use simple_interp::{Interpreter, InterpreterOutput, i64_into_buffer};
// use gc_heap::CopyingHeap;

// Get rid of some spurious VSCode errors
use core::option::Option;
use core::option::Option::None;
use core::prelude::rust_2024::derive;
use core::clone::Clone;
use core::cmp::{PartialEq,Eq};
use core::marker::Copy;

const FIRST_BORDER_ROW: usize = 1;
const LAST_BORDER_ROW: usize = BUFFER_HEIGHT - 1;
const TASK_MANAGER_WIDTH: usize = 10;
const TASK_MANAGER_BYTES: usize = BUFFER_HEIGHT * TASK_MANAGER_WIDTH;
const WINDOWS_WIDTH: usize = BUFFER_WIDTH - TASK_MANAGER_WIDTH;
const WINDOW_WIDTH: usize = (WINDOWS_WIDTH - 3) / 2;
const WINDOW_HEIGHT: usize = (LAST_BORDER_ROW - FIRST_BORDER_ROW - 2) / 2;
const MID_WIDTH: usize = WINDOWS_WIDTH / 2;
const MID_HEIGHT: usize = BUFFER_HEIGHT / 2;
const NUM_WINDOWS: usize = 4;

const FILENAME_PROMPT: &str = "F5 - Filename: ";

const MAX_OPEN: usize = 16;
const BLOCK_SIZE: usize = 256;
const NUM_BLOCKS: usize = 255;
const MAX_FILE_BLOCKS: usize = 64;
const MAX_FILE_BYTES: usize = MAX_FILE_BLOCKS * BLOCK_SIZE;
const MAX_FILES_STORED: usize = 30;
const MAX_FILENAME_BYTES: usize = 10;

const MAX_TOKENS: usize = 500;
const MAX_LITERAL_CHARS: usize = 30;
const STACK_DEPTH: usize = 50;
const MAX_LOCAL_VARS: usize = 20;
const HEAP_SIZE: usize = 1024;
const MAX_HEAP_BLOCKS: usize = HEAP_SIZE;

// Data type for a file system object:
// FileSystem<MAX_OPEN, BLOCK_SIZE, NUM_BLOCKS, MAX_FILE_BLOCKS, MAX_FILE_BYTES, MAX_FILES_STORED, MAX_FILENAME_BYTES>

// Data type for an interpreter object:
// Interpreter<MAX_TOKENS, MAX_LITERAL_CHARS, STACK_DEPTH, MAX_LOCAL_VARS, WINDOW_WIDTH, CopyingHeap<HEAP_SIZE, MAX_HEAP_BLOCKS>>

pub struct Window{
    vga:[[char; BUFFER_WIDTH-TASK_MANAGER_WIDTH];BUFFER_HEIGHT-1],
    in_use: bool,
    win_num: char,
}
impl Window {
    pub fn new(win_num: char) -> Self{
        let mut vga=[[' '; BUFFER_WIDTH-TASK_MANAGER_WIDTH];BUFFER_HEIGHT-1];
        Self{vga, in_use: false, win_num}
    }

    pub fn set_in_use(&mut self,val: bool){
        self.in_use = val;
    }

    pub fn update_borders(&mut self){
        let mut border_char = '.';
        if self.in_use{
            border_char = '*';
        }
        for i in 0..self.vga.len(){
            for j in 0..self.vga[0].len(){
                if i==0 || i == self.vga.len()-1 || j == 0 || j == self.vga[0].len()-1{
                    self.vga[i][j] = border_char;
                    if (j==0){
                        if (i==MID_WIDTH){
                            self.vga[i][j] = 'F'
                        }
                        else if (i==MID_WIDTH+1){
                            self.vga[i][j] = self.win_num;
                        }
                    }
                }
            }
        }
    }

}
pub struct Kernel {
    // YOUR CODE HERE
    screen: [[char; BUFFER_WIDTH]; BUFFER_HEIGHT],
    task_manager: [[char; TASK_MANAGER_WIDTH]; BUFFER_HEIGHT],
    top_row: [char; TASK_MANAGER_WIDTH],
    quad_f1: Window,
    quad_f2: Window,
    quad_f3: Window,
    quad_f4: Window,
    user_is_typing: bool,
    //user_input: &str,
    background_color: Color,
}

const HELLO: &str = r#"print("Hello, world!")"#;

const NUMS: &str = r#"print(1)
print(257)"#;

const ADD_ONE: &str = r#"x := input("Enter a number")
x := (x + 1)
print(x)"#;

const COUNTDOWN: &str = r#"count := input("count")
while (count > 0) {
    count := (count - 1)
}
print("done")
print(count)"#;

const AVERAGE: &str = r#"sum := 0
count := 0
averaging := true
while averaging {
    num := input("Enter a number:")
    if (num == "quit") {
        averaging := false
    } else {
        sum := (sum + num)
        count := (count + 1)
    }
}
print((sum / count))"#;

const PI: &str = r#"sum := 0
i := 0
neg := false
terms := input("Num terms:")
while (i < terms) {
    term := (1.0 / ((2.0 * i) + 1.0))
    if neg {
        term := -term
    }
    sum := (sum + term)
    neg := not neg
    i := (i + 1)
}
print((4 * sum))"#;

/*
// Seed the disk with some programs.
fn initial_files(disk: &mut FileSystem<MAX_OPEN, BLOCK_SIZE, NUM_BLOCKS, MAX_FILE_BLOCKS, MAX_FILE_BYTES, MAX_FILES_STORED, MAX_FILENAME_BYTES>) {
    for (filename, contents) in [
        ("hello", HELLO),
        ("nums", NUMS),
        ("add_one", ADD_ONE),
        ("countdown", COUNTDOWN),
        ("average", AVERAGE),
        ("pi", PI),
    ] {
        let fd = disk.open_create(filename).unwrap();
        disk.write(fd, contents.as_bytes()).unwrap();
        disk.close(fd);
    }
}
*/

impl Kernel {
    pub fn new() -> Self {
        let mut screen = [[' '; BUFFER_WIDTH]; BUFFER_HEIGHT];
        let mut task_manager= [[' '; TASK_MANAGER_WIDTH]; BUFFER_HEIGHT];
        let mut top_row= [' '; TASK_MANAGER_WIDTH];
        let mut quad_f1= Window::new('1');
        let mut quad_f2= Window::new('2');
        let mut quad_f3= Window::new('3');
        let mut quad_f4= Window::new('4');
        Self{screen, background_color: Color::Black, task_manager, top_row, quad_f1, quad_f2, quad_f3, quad_f4, user_is_typing: false}
    }

    pub fn key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::RawKey(code) => self.handle_raw(code),
            DecodedKey::Unicode(c) => self.handle_unicode(c)
        }
        self.draw();
    }
    pub fn update_in_use(&mut self, i: usize){
        self.quad_f1.set_in_use(false);
        self.quad_f2.set_in_use(false);
        self.quad_f3.set_in_use(false);
        self.quad_f4.set_in_use(false);
        
        if i == 1{
            self.quad_f1.set_in_use(true);
        }
        else if i == 2{
            self.quad_f2.set_in_use(true);
        }
        else if i == 3{
            self.quad_f3.set_in_use(true);
        }
        else if i == 4{
            self.quad_f4.set_in_use(true);
        }
    }
    fn handle_raw(&mut self, key: KeyCode) {
        match key{
            KeyCode::F1=> {
                self.update_in_use(1);
            }
            KeyCode::F2=> {
                self.update_in_use(2);
            }
            KeyCode::F3=> {
                self.update_in_use(3);
            }
            KeyCode::F4=> {
                self.update_in_use(4);
            }
            KeyCode::F5=> {
                self.update_in_use(0);
                self.user_is_typing = true;
            }

            _ => ()
        }
    }

    fn handle_unicode(&mut self, key: char) {
        match key {
            's' => {
                //self.user_input +='s';
            }
            _=>{}
        }
    }
    pub fn update_borders(&mut self){
        self.quad_f1.update_borders();
        self.quad_f2.update_borders();
        self.quad_f3.update_borders();
        self.quad_f4.update_borders();

    }
    pub fn update_screen(&mut self){
        for i in 0..BUFFER_WIDTH{
            for j in 0..BUFFER_HEIGHT{
                if j == 0{
                    self.screen[i][j] = self.top_row[i];
                }
                else if i >= BUFFER_WIDTH-TASK_MANAGER_WIDTH{
                    self.screen[i][j] = self.task_manager[i-(BUFFER_WIDTH-TASK_MANAGER_WIDTH)][j];
                }
                else{
                    if (i < WINDOW_WIDTH && j < WINDOW_HEIGHT){
                        self.screen[i][j] = self.quad_f1.vga[i%WINDOW_WIDTH][j%WINDOW_HEIGHT];
                    }
                    else if (i < WINDOW_WIDTH && j > WINDOW_HEIGHT){
                        self.screen[i][j] = self.quad_f3.vga[i%WINDOW_WIDTH][j%WINDOW_HEIGHT];
                    }
                    else if (i > WINDOW_WIDTH && j < WINDOW_HEIGHT){
                        self.screen[i][j] = self.quad_f2.vga[i%WINDOW_WIDTH][j%WINDOW_HEIGHT];
                    }
                    else if (i > WINDOW_WIDTH && j > WINDOW_HEIGHT){
                        self.screen[i][j] = self.quad_f4.vga[i%WINDOW_WIDTH][j%WINDOW_HEIGHT];
                    }
                }

            }
        }
    }
    pub fn draw(&mut self) {
        self.update_borders();
        self.update_screen();
        for i in 0..BUFFER_WIDTH{
            for j in 0..BUFFER_HEIGHT{
                plot(self.screen[i][j], i, j, ColorCode::new(Color::White, Color::Black));
            }
        }
        
    }

    pub fn draw_proc_status(&mut self) {
        todo!("Draw processor status");
    }

    pub fn run_one_instruction(&mut self) {
        todo!("Run an instruction in a process");
    }
}



fn text_color() -> ColorCode {
    ColorCode::new(Color::White, Color::Black)
}

fn highlight_color() -> ColorCode {
    ColorCode::new(Color::Black, Color::White)
}