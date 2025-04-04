#![no_std]
#![feature(prelude_2024)]

use file_system::FileSystem;
use gc_heap::CopyingHeap;
//use alloc::string::String;
// use file_system_solution::{FileSystem, FileSystemResult};
use pc_keyboard::{DecodedKey, KeyCode};
use pluggable_interrupt_os::println;
use pluggable_interrupt_os::vga_buffer::{BUFFER_WIDTH, BUFFER_HEIGHT, plot, ColorCode, Color, plot_str, is_drawable, plot_num};
use ramdisk::RamDisk;
use simple_interp::{Interpreter, InterpreterOutput, TickResult};
//use simple_interp::{Interpreter, InterpreterOutput, i64_into_buffer};
// use gc_heap::CopyingHeap;

// Get rid of some spurious VSCode errors
use core::option::Option;
use core::option::Option::None;
use core::panic;
use core::prelude::rust_2024::derive;
use core::clone::Clone;
use core::cmp::{PartialEq,Eq};
use core::marker::Copy;

const FIRST_BORDER_ROW: usize = 1;
const LAST_BORDER_ROW: usize = BUFFER_HEIGHT - 1;
const TASK_MANAGER_WIDTH: usize = 10;
const TASK_MANAGER_BYTES: usize = BUFFER_HEIGHT * TASK_MANAGER_WIDTH;
const WINDOWS_WIDTH: usize = BUFFER_WIDTH - TASK_MANAGER_WIDTH;
const WINDOW_WIDTH: usize = (WINDOWS_WIDTH) / 2; //had to change to work with struct
const WINDOW_HEIGHT: usize = ((LAST_BORDER_ROW - FIRST_BORDER_ROW) / 2) + 1; //had to change to work with struct
const MID_WIDTH: usize = WINDOWS_WIDTH / 2;
const MID_HEIGHT: usize = BUFFER_HEIGHT / 2;
const NUM_WINDOWS: usize = 4;

const FILENAME_PROMPT: &str = "F5 - Filename: ";
const F6: &str = "(F6)";

const MAX_OPEN: usize = 16;
const BLOCK_SIZE: usize = 256;
const NUM_BLOCKS: usize = 255;
const MAX_FILE_BLOCKS: usize = 8;  //Works as 8, but not as 64???
const MAX_FILE_BYTES: usize = MAX_FILE_BLOCKS * BLOCK_SIZE;
const MAX_FILES_STORED: usize = 30;
const MAX_FILENAME_BYTES: usize = 10;
/*
const MAX_OPEN: usize = 16;
const BLOCK_SIZE: usize = 256;
const NUM_BLOCKS: usize = 255;
const MAX_FILE_BLOCKS: usize = 64;
const MAX_FILE_BYTES: usize = MAX_FILE_BLOCKS * BLOCK_SIZE;
const MAX_FILES_STORED: usize = 30;
const MAX_FILENAME_BYTES: usize = 10; 
*/

const MAX_TOKENS: usize = 500;
const MAX_LITERAL_CHARS: usize = 30;
const STACK_DEPTH: usize = 50;
const MAX_LOCAL_VARS: usize = 20;
const HEAP_SIZE: usize = 1024;
const MAX_HEAP_BLOCKS: usize = HEAP_SIZE;

const MAX_USER_INPUT_BYTES: usize = MAX_FILENAME_BYTES + FILENAME_PROMPT.len();

// Data type for a file system object:
// FileSystem<MAX_OPEN, BLOCK_SIZE, NUM_BLOCKS, MAX_FILE_BLOCKS, MAX_FILE_BYTES, MAX_FILES_STORED, MAX_FILENAME_BYTES>

// Data type for an interpreter object:
// Interpreter<MAX_TOKENS, MAX_LITERAL_CHARS, STACK_DEPTH, MAX_LOCAL_VARS, WINDOW_WIDTH, CopyingHeap<HEAP_SIZE, MAX_HEAP_BLOCKS>>

pub struct Window{
    vga:[[char; WINDOW_WIDTH];WINDOW_HEIGHT],
    in_use: bool,
    win_num: char,
    contents:[[char;WINDOW_WIDTH-2];WINDOW_HEIGHT-2],
    foreground:[[Color; WINDOW_WIDTH];WINDOW_HEIGHT],
    background:[[Color; WINDOW_WIDTH];WINDOW_HEIGHT],
    current_highlighted: usize,
    is_being_edited: bool,
    current_contents_index: usize,
    current_file: [u8; MAX_FILENAME_BYTES],
    all_contents_u8: [u8; MAX_FILE_BYTES],
    current_u8_index: usize,
    
}
impl Clone for Window{
    fn clone(&self) -> Self {
        Self { vga: self.vga.clone(), in_use: self.in_use.clone(), win_num: self.win_num.clone(), contents: self.contents.clone(), foreground: self.foreground.clone(), background: self.background.clone(), current_highlighted: self.current_highlighted.clone(), is_being_edited: self.is_being_edited.clone(), current_contents_index: self.current_contents_index.clone(), current_file: self.current_file.clone(), all_contents_u8: self.all_contents_u8.clone(), current_u8_index: self.current_u8_index.clone() }
    }
}
impl Copy for Window{

}
impl Window {
    pub fn new(win_num: char) -> Self{
        let mut vga=[['\0'; WINDOW_WIDTH];WINDOW_HEIGHT];
        let mut contents = [['\0';WINDOW_WIDTH-2]; WINDOW_HEIGHT-2];
        let mut foreground=[[Color::White; WINDOW_WIDTH];WINDOW_HEIGHT];
        let mut background=[[Color::Black; WINDOW_WIDTH];WINDOW_HEIGHT];
        let mut current_file = ['*' as u8;MAX_FILENAME_BYTES];
        let mut all_contents_u8 = ['\0' as u8; MAX_FILE_BYTES];
        Self{vga, in_use: false, win_num, contents, foreground, background, current_highlighted: 0, is_being_edited: false, current_file, current_contents_index: 0, all_contents_u8, current_u8_index: 0}
    }



    pub fn set_in_use(&mut self,val: bool){
        self.in_use = val;
    }
    pub fn update_contents(&mut self, c: [[char;WINDOW_WIDTH-2];WINDOW_HEIGHT-2]){
        self.contents = c;
        for i in 1..WINDOW_HEIGHT-1{
            for j in 1..WINDOW_WIDTH-1{
                self.vga[i][j] = self.contents[i-1][j-1];
            }
        }
    }
    pub fn find_u8_index(&mut self){
        let mut type_index = MAX_FILE_BYTES;
        for i in self.all_contents_u8.iter().rev(){
            
            if *i as char !='\0'{
                break;
                
            
            }
            type_index -=1;
        }


        // let mut type_index = 0;
        // for i in self.all_contents_u8{
        //     if i as char == '\0'{
        //         break;
        //     }
        //     type_index+=1;
        // }
        self.current_u8_index = type_index;
    }
    pub fn find_contents_index(&mut self){
        let mut type_index = (WINDOW_WIDTH-2)*(WINDOW_HEIGHT-2);
        let mut found = false;
        for i in self.contents.iter().rev(){
            if found{
                break;
            }
            for j in i.iter().rev(){
                if *j !='\0'{
                    found = true;
                    break;
                }
                type_index -=1;
            }
        }
        // println!("{:?}",self.contents);
        // println!("{}",type_index);
        


        // let mut type_index = 0;
        // for i in self.contents{
        //     for j in i{
        //         if j =='\0'{
        //             break;
        //         }
        //         type_index+=1;
        //     }
        // }
        self.current_contents_index = type_index;
        if self.current_contents_index < 0{
            self.current_contents_index = 0
        }
    }
    pub fn edit_press_enter(&mut self){
        let row = (self.current_contents_index / (WINDOW_WIDTH-2)) + 1;
        if row < WINDOW_HEIGHT-2 {
            let mut count = 0;
            //self.type_char(' ');
            // for i in (self.current_contents_index+1)..(row*(WINDOW_WIDTH-2)+2){
            //     self.current_contents_index+=1;
            //     self.current_u8_index+=1;
            //     self.type_char('\0');
                
            //     count +=1;
            // }

            self.current_contents_index =(row*(WINDOW_WIDTH-2)+1);
            self.type_char('\n');
            // self.current_u8_index-=2;
            self.current_contents_index-=2;
            //self.current_contents_index = self.current_contents_index + count;
            //println!("{}", count);
            //panic!();
        }
        
    }
    pub fn type_char(&mut self, c: char){  //backspace error dont increment
        if self.current_contents_index != (WINDOW_WIDTH-2)*(WINDOW_HEIGHT-2){
            if c=='\n'{
                self.contents[self.current_contents_index/(WINDOW_WIDTH-2)][self.current_contents_index%(WINDOW_WIDTH-2)] = ' ';
            }
            else{
                self.contents[self.current_contents_index/(WINDOW_WIDTH-2)][self.current_contents_index%(WINDOW_WIDTH-2)] = c;
            }
            //self.contents[self.current_contents_index/(WINDOW_WIDTH-2)][self.current_contents_index%(WINDOW_WIDTH-2)] = c;
            self.all_contents_u8[self.current_u8_index] = c as u8;
            self.update_contents(self.contents);
            if (c!='\0'){
                self.current_contents_index+=1;
                self.current_u8_index +=1;
            }
            else{
                if self.current_contents_index > 1{
                    if (self.contents[(self.current_contents_index-1)/(WINDOW_WIDTH-2)][(self.current_contents_index-1)%(WINDOW_WIDTH-2)] == '\0' || (self.contents[(self.current_contents_index-1)/(WINDOW_WIDTH-2)][(self.current_contents_index-1)%(WINDOW_WIDTH-2)] == ' ' && self.contents[(self.current_contents_index-2)/(WINDOW_WIDTH-2)][(self.current_contents_index-2)%(WINDOW_WIDTH-2)] == '\0')) && (self.current_contents_index)%(WINDOW_WIDTH-2) !=0{
                        self.current_contents_index-=1;
                        self.type_char('\0');
                    }
                }
                self.current_u8_index = self.current_contents_index;
            }
        }
    }
    pub fn reset_colors(&mut self){
        self.foreground = [[Color::White; WINDOW_WIDTH];WINDOW_HEIGHT];
        self.background = [[Color::Black; WINDOW_WIDTH];WINDOW_HEIGHT];
    }
    pub fn update_colors(&mut self){
        self.reset_colors();
        let row = ((self.current_highlighted) /3) + 1;
        let col = self.current_highlighted % 3;
        for i in 0..MAX_FILENAME_BYTES{
            self.foreground[row][((col*10)+1) + i] = Color::Black;
            self.background[row][((col*10)+1) + i] = Color::White;
        }
    }
    pub fn update_borders(&mut self){
        let mut border_char = '.';
        if self.in_use{ 
            border_char = '*';
        } 
        for i in 0..WINDOW_HEIGHT{ 
            for j in 0..WINDOW_WIDTH{
                if i==0 || i == WINDOW_HEIGHT-1 || j == 0 || j == WINDOW_WIDTH-1{
                    self.vga[i][j] = border_char;
                    if (i==0){
                        if (j==MID_WIDTH/2){
                            self.vga[i][j] = 'F'
                        }
                        else if (j==(MID_WIDTH/2)+1){
                            self.vga[i][j] = self.win_num;
                        }
                    }
                }
            }
        }
        if self.is_being_edited{
            self.start_editing();
        }
    }

    pub fn start_editing(&mut self){
        self.reset_colors();
            for i in 2..MAX_FILENAME_BYTES+4{
                if i < 6{
                    self.vga[0][i] = F6.as_bytes()[i-2] as char;
                }
                else{
                    let a = self.current_file[i-6] as char;
                    if a.is_alphanumeric(){
                        self.vga[0][i] = self.current_file[i-6] as char;
                        self.foreground[0][i] = Color::Black;
                        self.background[0][i] = Color::White
                    }
                }
            }

    }

}

// pub struct Output{

// }

// impl InterpreterOutput for Output{
//     fn new() -> Self{
//         Self {  }
//     }
// }
pub struct Kernel {
    // YOUR CODE HERE
    screen: [[char; BUFFER_WIDTH]; BUFFER_HEIGHT],
    task_manager: [[char; TASK_MANAGER_WIDTH]; BUFFER_HEIGHT],
    top_row: [char; BUFFER_WIDTH],
    quad_f1: Window,
    quad_f2: Window,
    quad_f3: Window,
    quad_f4: Window,
    user_is_typing: bool,
    in_use: usize,
    user_input: [char; MAX_USER_INPUT_BYTES],
    current_user_input_index: usize,
    background_color: Color,
    filesystem: FileSystem<MAX_OPEN, BLOCK_SIZE, NUM_BLOCKS, MAX_FILE_BLOCKS, MAX_FILE_BYTES, MAX_FILES_STORED, MAX_FILENAME_BYTES>,
    //output: Output,
    //current_processes: [Interpreter<MAX_TOKENS, MAX_LITERAL_CHARS, STACK_DEPTH, MAX_LOCAL_VARS, WINDOW_WIDTH, CopyingHeap<HEAP_SIZE,MAX_HEAP_BLOCKS>>; 4],
    total_ticks: usize,
    //current_outputs: [Output; 4],
    is_blocked: [bool;4],
    is_running: [bool;4],
    instructions_executed: [usize; 4],
    foreground: [[Color; BUFFER_WIDTH];BUFFER_HEIGHT],
    background: [[Color; BUFFER_WIDTH];BUFFER_HEIGHT],
    editing: bool,
    bool_f1: (bool, bool),
    bool_f2: (bool, bool),
    bool_f3: (bool, bool),
    //bool_f4: (bool, bool),
    int_f1: Interpreter<MAX_TOKENS, MAX_LITERAL_CHARS, STACK_DEPTH, MAX_LOCAL_VARS, WINDOW_WIDTH, CopyingHeap<HEAP_SIZE, MAX_HEAP_BLOCKS>>,
    int_f2: Interpreter<MAX_TOKENS, MAX_LITERAL_CHARS, STACK_DEPTH, MAX_LOCAL_VARS, WINDOW_WIDTH, CopyingHeap<HEAP_SIZE, MAX_HEAP_BLOCKS>>,
    int_f3: Interpreter<MAX_TOKENS, MAX_LITERAL_CHARS, STACK_DEPTH, MAX_LOCAL_VARS, WINDOW_WIDTH, CopyingHeap<HEAP_SIZE, MAX_HEAP_BLOCKS>>,
    running: bool,
    waiting: bool,
    input_flag: bool,
    input_flag1: bool,
    input_flag2: bool,
    input_flag3: bool,
    run_input: [char; 20],
    ri_index: usize,
    process_ran: usize,
    turn_index: usize,
    num_ticks: [usize; 4],
    
    //int_f4: Interpreter<MAX_TOKENS, MAX_LITERAL_CHARS, STACK_DEPTH, MAX_LOCAL_VARS, WINDOW_WIDTH, CopyingHeap<HEAP_SIZE, MAX_HEAP_BLOCKS>>,
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
When writing in file do we need to worry about enter button? YES

Is basic round robin good enough or do we need to do a priority queue. basic

By create data type, do you suggest a parameter in Kernel or a seperate struct. Struct

Look for \n explicitly

 */
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
    //panic!();
}


impl Kernel {
    pub fn new() -> Self {
        let mut screen = [['\0'; BUFFER_WIDTH]; BUFFER_HEIGHT];
        let mut task_manager= [['\0'; TASK_MANAGER_WIDTH]; BUFFER_HEIGHT];
        let mut top_row= ['\0'; BUFFER_WIDTH];
        let mut quad_f1= Window::new('1');
        let mut quad_f2= Window::new('2');
        let mut quad_f3= Window::new('3');
        let mut quad_f4= Window::new('4');
        let mut in_use = 0;
        let mut user_input = ['\0'; MAX_USER_INPUT_BYTES];
        for (i,c) in FILENAME_PROMPT.chars().enumerate(){
            user_input[i] = c;
        }
        let mut filesystem: FileSystem<MAX_OPEN, BLOCK_SIZE, NUM_BLOCKS, MAX_FILE_BLOCKS, MAX_FILE_BYTES, MAX_FILES_STORED, MAX_FILENAME_BYTES> = FileSystem::new(ramdisk::RamDisk::new());
        //let mut current_processes:Interpreter<MAX_TOKENS, MAX_LITERAL_CHARS, STACK_DEPTH, MAX_LOCAL_VARS, WINDOW_WIDTH, CopyingHeap<HEAP_SIZE,MAX_HEAP_BLOCKS>> = [Interpreter::new(""), 4];
        //let mut current_outputs = [Output::new(); 4];
        let mut is_blocked = [true;4];
        let mut is_running = [false;4];
        let mut instructions_executed = [0;4];

        let mut foreground = [[Color::White;BUFFER_WIDTH];BUFFER_HEIGHT];
        let mut background = [[Color::Black;BUFFER_WIDTH];BUFFER_HEIGHT];

        //Self{screen, background_color: Color::Black, task_manager, top_row, quad_f1, quad_f2, quad_f3, quad_f4, user_is_typing: false, in_use, user_input, current_user_input_index: FILENAME_PROMPT.len(), filesystem, output: Output::new(), current_processes, current_outputs, total_ticks: 0, is_blocked, is_running, instructions_exectued }
        let int_f1 = Interpreter::new("");
        let int_f2 = Interpreter::new("");
        let int_f3 = Interpreter::new("");
        //let int_f4 = Interpreter::new("");
        let mut bool_f1 = (false, false);
        let mut bool_f2 = (false, false);
        let mut bool_f3 = (false, false);
        let mut input_flag = false;
        let mut run_input = ['\0'; 20];
        let mut num_ticks = [0;4];
        //let mut bool_f4 = (false, false);
        Self{screen, background_color: Color::Black, task_manager, top_row, quad_f1, quad_f2, quad_f3, quad_f4, user_is_typing: false, in_use, user_input, current_user_input_index: FILENAME_PROMPT.len(), filesystem,  total_ticks: 0, is_blocked, is_running,instructions_executed, foreground, background, editing: false, int_f1, int_f2, int_f3, bool_f1,bool_f2,bool_f3, running: false, waiting: false, input_flag, run_input, ri_index: 0, process_ran:0,turn_index:0,num_ticks,input_flag1: false, input_flag2: false, input_flag3: false}

    }

    pub fn make_initial_files(&mut self){
        // let f1 = self.filesystem.open_create("hello").unwrap();
        // let f2 = self.filesystem.open_create("nums").unwrap();
        // let f3= self.filesystem.open_create("average").unwrap();
        // let f4 = self.filesystem.open_create("pi").unwrap();
        // let f5 = self.filesystem.open_create("countdown").unwrap();
        // let f6 = self.filesystem.open_create("addOne").unwrap();



        // self.filesystem.write(f1, HELLO.as_bytes()).unwrap();
        // self.filesystem.close(f1);

        // self.filesystem.write(f2, NUMS.as_bytes()).unwrap();
        // self.filesystem.close(f2);

        // self.filesystem.write(f3, AVERAGE.as_bytes()).unwrap();
        // self.filesystem.close(f3);

        // self.filesystem.write(f4, PI.as_bytes()).unwrap();
        // self.filesystem.close(f4);

        // self.filesystem.write(f5, COUNTDOWN.as_bytes()).unwrap();
        // self.filesystem.close(f5);

        // self.filesystem.write(f6, ADD_ONE.as_bytes()).unwrap();
        // self.filesystem.close(f6);

        initial_files(&mut self.filesystem);


        // let u8_name =self.filesystem.list_directory().unwrap().1[3];
        // let str_name = core::str::from_utf8(&u8_name).unwrap();
        // println!("{str_name}");
        // let fd = self.filesystem.open_read("pi").unwrap();
        // println!("{fd}");
        // let mut content_buffer = ['\0' as u8;MAX_FILE_BYTES];
        // let mut window_content_buffer = ['\0'; (WINDOW_WIDTH-2) * (WINDOW_HEIGHT-2)];
        // let contents = self.filesystem.read(fd, &mut content_buffer).unwrap();
        // self.filesystem.close(fd);
        // for i in 0..((WINDOW_WIDTH-2) * (WINDOW_HEIGHT-2)){
        //     window_content_buffer[i] = content_buffer[i] as char;
        // }
        // println!("{:?}",window_content_buffer);
        // panic!();
       
        // let mut content_buff = ['\0'; (WINDOW_WIDTH-2)*(WINDOW_HEIGHT-2)];
        // let mut char_count = 0;

        // for i in self.filesystem.list_directory().unwrap().1{
        //     for j in i{
        //         content_buff[char_count] = j as char;
        //         char_count +=1;
        //     }
            
        // }
        for i in 1..5 as usize{
            self.in_use = i;
            self.default_window();
        }
        self.in_use = 0;
        



    }

    pub fn write_to_window(&mut self, window_num: usize, contents: [char; (WINDOW_WIDTH-2)*(WINDOW_HEIGHT-2)]){


        let mut char_count = 0;
        let mut row_count = 0;
        let mut c = [['\0';WINDOW_WIDTH-2];WINDOW_HEIGHT-2];
        for i in contents{
            if i=='\n'{
                
                char_count = 0;
                row_count +=1;
                continue;
            }
            let mut val = 5;
            if (self.editing){
                val = 2;
            }
            if char_count >= WINDOW_WIDTH-val{
                char_count = 0;
                row_count +=1;
            }
            if row_count >=WINDOW_HEIGHT-2{
                break;
            }
            c[row_count][char_count] = i;
            char_count+=1;
            
            
        }



        if window_num ==1{
            self.quad_f1.update_contents(c);
            self.quad_f1.find_contents_index(); 
        }
        if window_num ==2{
            self.quad_f2.update_contents(c);
            self.quad_f2.find_contents_index(); 
        }
        if window_num ==3{
            self.quad_f3.update_contents(c);
            self.quad_f3.find_contents_index(); 
        }
        if window_num ==4{
            self.quad_f4.update_contents(c);
            self.quad_f4.find_contents_index(); 
        }
        self.update_screen();
        //println!("{:?}", contents);
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
        self.in_use = i;
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
        self.update_borders();
        self.update_screen();
    }



    fn handle_raw(&mut self, key: KeyCode) {
        match key{
            KeyCode::F1=> {
                if !self.editing{
                    self.update_in_use(1);
                    self.user_is_typing = false;
                }
            }
            KeyCode::F2=> {
                if !self.editing{
                    self.update_in_use(2);
                    self.user_is_typing = false;
                }
            }
            KeyCode::F3=> {
                if !self.editing{
                    self.update_in_use(3);
                    self.user_is_typing = false;
                }
            }
            KeyCode::F4=> {
                if !self.editing{
                    self.update_in_use(4);
                    self.user_is_typing = false;
                }
            }
            KeyCode::F5=> {
                if !self.editing {
                    self.update_in_use(0);
                    self.user_is_typing = true;
                }
            }
            KeyCode::F6=>{
                // if self.in_use !=0{
                //     if self.is_running(self.in_use-1){
                //         self.is_blocked = true;
                //         //return to file selector screen
                //     }
                // }
                if self.editing{
                    self.shut_off_editing();
                }
                if self.running{
                    
                    self.running = false;
                    if self.in_use == 1{
                        self.quad_f1.is_being_edited =false;
                        self.bool_f1 = (false, false);
                        self.input_flag1 = false;
                        self.wait_check();
                        self.ri_index = 0;
                        self.run_input = ['\0';20];
                        let empty = ['\0';(WINDOW_WIDTH-2)*(WINDOW_HEIGHT-2)];
                        self.write_to_window(1, empty);
                        self.num_ticks[0] = 0;

                    }
                    else if self.in_use == 2{
                        self.bool_f2 = (false, false);
                        self.input_flag2 = false;
                        self.wait_check();
                        self.ri_index = 0;
                        self.run_input = ['\0';20];
                        let empty = ['\0';(WINDOW_WIDTH-2)*(WINDOW_HEIGHT-2)];
                        self.write_to_window(2, empty);
                        self.num_ticks[1] = 0;
                    }
                    else if self.in_use == 3{
                        self.bool_f3 = (false, false);
                        self.input_flag3 = false;
                        self.wait_check();
                        self.ri_index = 0;
                        self.run_input = ['\0';20];
                        let empty = ['\0';(WINDOW_WIDTH-2)*(WINDOW_HEIGHT-2)];
                        self.write_to_window(3, empty);
                        self.num_ticks[2] = 0;
                    }
                    self.default_window();
                }
                else{
                    if self.in_use ==1{
                        self.num_ticks[0]=0;
                    }
                    else if self.in_use ==2{
                        self.num_ticks[1]=0;
                    }
                    else if self.in_use ==3{
                        self.num_ticks[2] = 0;
                    }
                    self.default_window();
                }
            }
            KeyCode::ArrowLeft=>{
                if !self.editing{
                    self.move_left();
                }
            }
            KeyCode::ArrowRight=>{
                 
                if !self.editing{
                    self.move_right();
                }
            }
            KeyCode::ArrowDown=>{
               
                if !self.editing{
                    self.move_down();
                }
            }
            KeyCode::ArrowUp=>{
                
                if !self.editing&& !self.running{
                    self.move_up();
                }
            }
            

            _ => ()
        }
    }


    pub fn default_window(&mut self){
        let mut content_buff = ['\0'; (WINDOW_WIDTH-2)*(WINDOW_HEIGHT-2)];
        let mut char_count = 0;
        for i in self.filesystem.list_directory().unwrap().1{
            for j in i{
                content_buff[char_count] = j as char;
                char_count +=1;
            }
            
        }

        if self.in_use ==1{
            self.quad_f1.is_being_edited = false;
            self.quad_f1.reset_colors();
            self.quad_f1.update_borders();
            self.quad_f1.find_contents_index();
            self.write_to_window(1, content_buff);
        }
        else if self.in_use ==2{
            self.quad_f2.is_being_edited = false;
            self.quad_f2.reset_colors();
            self.quad_f2.update_borders();
            self.quad_f2.find_contents_index();
            self.write_to_window(2, content_buff);
        }
        else if self.in_use ==3{
            self.quad_f3.is_being_edited = false;
            self.quad_f3.reset_colors();
            self.quad_f3.update_borders();
            self.quad_f3.find_contents_index();
            self.write_to_window(3, content_buff);
        }
        else if self.in_use ==4{
            self.quad_f4.is_being_edited = false;
            self.quad_f4.reset_colors();
            self.quad_f4.update_borders();
            self.quad_f4.find_contents_index();
            self.write_to_window(3, content_buff);
        }
    }
    pub fn shut_off_editing(&mut self){
        if self.in_use==1{
            let filename = core::str::from_utf8(&self.quad_f1.current_file).unwrap();
            let fd = self.filesystem.open_create(filename).unwrap();
            
            self.filesystem.write(fd, &self.quad_f1.all_contents_u8);
            self.filesystem.close(fd);
            self.editing = false;
            let mut content_buff = ['\0'; (WINDOW_WIDTH-2)*(WINDOW_HEIGHT-2)];
            let mut char_count = 0;

            for i in self.filesystem.list_directory().unwrap().1{
                for j in i{
                    content_buff[char_count] = j as char;
                    char_count +=1;
                }
                
            }
            self.quad_f1.is_being_edited = false;
            self.quad_f1.reset_colors();
            self.quad_f1.update_borders();
            self.quad_f1.find_contents_index();
            //panic!();
            self.write_to_window(1, content_buff);
            
        }
        else if self.in_use==2{
            let filename = core::str::from_utf8(&self.quad_f2.current_file).unwrap();
            let fd = self.filesystem.open_create(filename).unwrap();
            self.filesystem.write(fd, &self.quad_f2.all_contents_u8);
            self.filesystem.close(fd);
            self.editing = false;
            let mut content_buff = ['\0'; (WINDOW_WIDTH-2)*(WINDOW_HEIGHT-2)];
            let mut char_count = 0;

            for i in self.filesystem.list_directory().unwrap().1{
                for j in i{
                    content_buff[char_count] = j as char;
                    char_count +=1;
                }
                
            }
            self.quad_f2.is_being_edited = false;
            self.quad_f2.reset_colors();
            self.quad_f2.update_borders();
            self.write_to_window(2, content_buff);
            
        }
        else if self.in_use==3{
            let filename = core::str::from_utf8(&self.quad_f3.current_file).unwrap();
            let fd = self.filesystem.open_create(filename).unwrap();
            self.filesystem.write(fd, &self.quad_f3.all_contents_u8);
            self.filesystem.close(fd);
            self.editing = false;
            let mut content_buff = ['\0'; (WINDOW_WIDTH-2)*(WINDOW_HEIGHT-2)];
            let mut char_count = 0;

            for i in self.filesystem.list_directory().unwrap().1{
                for j in i{
                    content_buff[char_count] = j as char;
                    char_count +=1;
                }
                
            }
            self.quad_f3.is_being_edited = false;
            self.quad_f3.reset_colors();
            self.quad_f3.update_borders();
            self.write_to_window(3, content_buff);
            
        }
        else if self.in_use==4{
            let filename = core::str::from_utf8(&self.quad_f4.current_file).unwrap();
            let fd = self.filesystem.open_create(filename).unwrap();
            self.filesystem.write(fd, &self.quad_f4.all_contents_u8);
            self.filesystem.close(fd);
            self.editing = false;
            let mut content_buff = ['\0'; (WINDOW_WIDTH-2)*(WINDOW_HEIGHT-2)];
            let mut char_count = 0;

            for i in self.filesystem.list_directory().unwrap().1{
                for j in i{
                    content_buff[char_count] = j as char;
                    char_count +=1;
                }
                
            }
            self.quad_f4.is_being_edited = false;
            self.quad_f4.reset_colors();
            self.quad_f4.update_borders();
            self.write_to_window(4, content_buff);
            
        }
    }

    pub fn move_left(&mut self){
        if self.in_use ==1{
            if self.quad_f1.current_highlighted >0{
                self.quad_f1.current_highlighted-=1;
                self.quad_f1.update_colors();
            }
        }
        else if self.in_use ==2{
            if self.quad_f2.current_highlighted >0{
                self.quad_f2.current_highlighted-=1;
                self.quad_f2.update_colors();
            }
        }
        else if self.in_use ==3{
            if self.quad_f3.current_highlighted >0{
                self.quad_f3.current_highlighted-=1;
                self.quad_f3.update_colors();
            }
        }
        else if self.in_use ==4{
            if self.quad_f4.current_highlighted >0{
                self.quad_f4.current_highlighted-=1;
                self.quad_f4.update_colors();
            }
        }
    }
    pub fn move_right(&mut self){
        if self.in_use ==1{
            if self.quad_f1.current_highlighted <MAX_FILES_STORED-1{
                self.quad_f1.current_highlighted+=1;
                self.quad_f1.update_colors();
            }
        }
        else if self.in_use ==2{
            if self.quad_f2.current_highlighted <MAX_FILES_STORED-1{
                self.quad_f2.current_highlighted+=1;
                self.quad_f2.update_colors();
            }
        }
        else if self.in_use ==3{
            if self.quad_f3.current_highlighted <MAX_FILES_STORED-1{
                self.quad_f3.current_highlighted+=1;
                self.quad_f3.update_colors();
            }
        }
        else if self.in_use ==4{
            if self.quad_f4.current_highlighted <MAX_FILES_STORED-1{
                self.quad_f4.current_highlighted+=1;
                self.quad_f4.update_colors();
            }
        }
    }
    pub fn move_down(&mut self){
        if self.in_use ==1{
            if self.quad_f1.current_highlighted <MAX_FILES_STORED-2-1{
                self.quad_f1.current_highlighted+=3;
                self.quad_f1.update_colors();
            }
        }
        else if self.in_use ==2{
            if self.quad_f2.current_highlighted <MAX_FILES_STORED-2-1{
                self.quad_f2.current_highlighted+=3;
                self.quad_f2.update_colors();
            }
        }
        else if self.in_use ==3{
            if self.quad_f3.current_highlighted <MAX_FILES_STORED-2-1{
                self.quad_f3.current_highlighted+=3;
                self.quad_f3.update_colors();
            }
        }
        else if self.in_use ==4{
            if self.quad_f4.current_highlighted <MAX_FILES_STORED-2-1{
                self.quad_f4.current_highlighted+=3;
                self.quad_f4.update_colors();
            }
        }
    }
    pub fn move_up(&mut self){
        if self.in_use ==1{
            if self.quad_f1.current_highlighted >2{
                self.quad_f1.current_highlighted-=3;
                self.quad_f1.update_colors();
            }
        }
        else if self.in_use ==2{
            if self.quad_f2.current_highlighted >2{
                self.quad_f2.current_highlighted-=3;
                self.quad_f2.update_colors();
            }
        }
        else if self.in_use ==3{
            if self.quad_f3.current_highlighted >2{
                self.quad_f3.current_highlighted-=3;
                self.quad_f3.update_colors();
            }
        }
        else if self.in_use ==4{
            if self.quad_f4.current_highlighted >2{
                self.quad_f4.current_highlighted-=3;
                self.quad_f4.update_colors();
            }
        }
    }

    fn handle_unicode(&mut self, key: char) {
        if self.user_is_typing && key.is_alphanumeric(){  //Probably need to change is_alphanumeric
            if self.current_user_input_index < (MAX_USER_INPUT_BYTES){
                self.user_input[self.current_user_input_index] = key;
                self.current_user_input_index +=1;
                self.update_screen();
            }
        }
        else if self.user_is_typing && self.current_user_input_index>FILENAME_PROMPT.len() && key=='\u{8}'{  
            self.user_input[self.current_user_input_index-1] = '\0';
            self.current_user_input_index -=1;
            self.update_screen();
        }
        else if (self.user_is_typing && key=='\n'){
            let mut temp_buff = [0 as u8; MAX_FILENAME_BYTES];
            let p_name = &self.user_input[FILENAME_PROMPT.len()..self.user_input.len()];

            for (i,c) in p_name.iter().enumerate(){
                temp_buff[i] = *c as u8;
            }
            let mut program = core::str::from_utf8(&temp_buff).unwrap();
            println!("{:?}",program);
            let fd = self.filesystem.open_create(program).unwrap();
            println!("1");
            self.filesystem.close(fd);
            for i in 0..MAX_FILENAME_BYTES{
                self.handle_unicode('\u{8}');
            }
            let mut content_buff = ['\0'; (WINDOW_WIDTH-2)*(WINDOW_HEIGHT-2)];
            let mut char_count = 0;
            for i in self.filesystem.list_directory().unwrap().1{
                for j in i{
                    content_buff[char_count] = j as char;
                    char_count +=1;
                }
                
            }
            for i in 1..5 as usize{
                self.write_to_window(i, content_buff);
            }
            
            // TODO WEIRD ERRROR MINUS 
        } 
        
        else if (!self.user_is_typing && !self.editing && key=='r'){
            self.run_file();
        }
        else if (key=='e' && !self.editing){
            self.edit_file_setup();
        }

        else if (self.editing){
            self.edit_file_text(key);
        }
        self.wait_check();
        if (self.waiting){
            if key=='\n'  { 
                if self.in_use == 1{
                    self.bool_f1.1 = false;
                    self.quad_f1.edit_press_enter();
                    self.input_flag1 = true;
                }
                //else if ...
                else if self.in_use == 2{
                    self.bool_f2.1 = false;
                    self.quad_f2.edit_press_enter();
                    self.input_flag2 = true;
                }
                else if self.in_use == 3{
                    self.bool_f3.1 = false;
                    self.quad_f3.edit_press_enter();
                    self.input_flag3 = true;
                }
                self.wait_check();
                
            }
            else if (is_drawable(key)){
                if (self.ri_index<self.run_input.len()){
                    self.run_input[self.ri_index] = key;
                    self.ri_index +=1;
                    self.edit_file_text(key);
                }
                
            }
            else if key == '\u{8}' {
                if self.ri_index > 0{
                    self.ri_index-=1;
                    self.run_input[self.ri_index] = '\0';
                    self.edit_file_text(key);
                }
            }
        }


    }

    pub fn wait_check(&mut self){
        if !self.bool_f1.1 && !self.bool_f2.1 && !self.bool_f3.1{ // and others{}
                    self.waiting = false;
                }
    }

    pub fn edit_file_text(&mut self, key: char){
        if self.in_use==1{
            if key=='\u{8}' {
                if (self.quad_f1.current_contents_index > 0){
                    self.quad_f1.find_u8_index();
                    self.quad_f1.current_contents_index = self.quad_f1.current_contents_index-1;
                    self.quad_f1.current_u8_index = self.quad_f1.current_u8_index-1;
                    
                    self.quad_f1.type_char('\0');
                }
            }
            else if key=='\n'{
                self.quad_f1.edit_press_enter();
            }
            else{
                //self.quad_f1.find_contents_index(); //some goofy shit here
                self.quad_f1.type_char(key);
                self.update_screen();
            }
        }
        else if self.in_use==2{
            if key=='\u{8}'{
                //self.quad_f1.find_contents_index();
                if (self.quad_f2.current_contents_index > 0){
                    self.quad_f2.find_u8_index();
                    self.quad_f2.current_contents_index = self.quad_f2.current_contents_index-1;
                    self.quad_f2.current_u8_index = self.quad_f2.current_u8_index-1;
                    
                    self.quad_f2.type_char('\0');
                }
            }
            else if key=='\n'{
                self.quad_f2.edit_press_enter();
            }
            else{
                //self.quad_f1.find_contents_index(); //some goofy shit here
                self.quad_f2.type_char(key);
                self.update_screen();
            }
        }
        else if self.in_use==3{
            if key=='\u{8}'{
                //self.quad_f1.find_contents_index();
                if (self.quad_f3.current_contents_index > 0){
                    self.quad_f3.find_u8_index();
                    self.quad_f3.current_contents_index = self.quad_f3.current_contents_index-1;
                    self.quad_f3.current_u8_index = self.quad_f3.current_u8_index-1;
                    
                    self.quad_f3.type_char('\0');
                }
            }
            else if key=='\n'{
                self.quad_f3.edit_press_enter();
            }
            else{
                //self.quad_f1.find_contents_index(); //some goofy shit here
                self.quad_f3.type_char(key);
                self.update_screen();
            }
        }
        else if self.in_use==4{
            if key=='\u{8}'{
                if (self.quad_f4.current_contents_index > 0){
                    //self.quad_f1.find_contents_index();
                    self.quad_f4.find_u8_index();
                    self.quad_f4.current_contents_index = self.quad_f4.current_contents_index-1;
                    self.quad_f4.current_u8_index = self.quad_f4.current_u8_index-1;
                    
                    self.quad_f4.type_char('\0');
                }
            }
            else if key=='\n'{
                self.quad_f4.edit_press_enter();
            }
            else{
                //self.quad_f1.find_contents_index(); //some goofy shit here
                self.quad_f4.type_char(key);
                self.update_screen();
            }
        }
    }

    pub fn edit_file_setup(&mut self){
        if self.in_use == 1{
            self.quad_f1.is_being_edited = true;
            let u8_name =self.filesystem.list_directory().unwrap().1[self.quad_f1.current_highlighted];
            self.quad_f1.current_file = u8_name;
            self.editing = true;
            let str_name = core::str::from_utf8(&u8_name).unwrap();
            let fd = self.filesystem.open_read(str_name).unwrap();
            let mut content_buffer = ['\0' as u8;MAX_FILE_BYTES];
            
            let mut window_content_buffer = ['\0'; (WINDOW_WIDTH-2) * (WINDOW_HEIGHT-2)];
            let contents = self.filesystem.read(fd, &mut content_buffer).unwrap();
            self.quad_f1.all_contents_u8 = content_buffer;
            self.quad_f1.find_u8_index();
            self.filesystem.close(fd);
            for i in 0..((WINDOW_WIDTH-2) * (WINDOW_HEIGHT-2)){
                
                window_content_buffer[i] = content_buffer[i] as char;
            }
            self.write_to_window(1, window_content_buffer)
        }
        else if self.in_use == 2{
            self.quad_f2.is_being_edited = true;
            let u8_name =self.filesystem.list_directory().unwrap().1[self.quad_f2.current_highlighted];
            self.quad_f1.current_file = u8_name;
            self.editing = true;
            let str_name = core::str::from_utf8(&u8_name).unwrap();
            let fd = self.filesystem.open_read(str_name).unwrap();
            let mut content_buffer = ['\0' as u8;MAX_FILE_BYTES];
            
            let mut window_content_buffer = ['\0'; (WINDOW_WIDTH-2) * (WINDOW_HEIGHT-2)];
            let contents = self.filesystem.read(fd, &mut content_buffer).unwrap();
            self.quad_f2.all_contents_u8 = content_buffer;
            self.quad_f2.find_u8_index();
            self.filesystem.close(fd);
            for i in 0..((WINDOW_WIDTH-2) * (WINDOW_HEIGHT-2)){
                
                window_content_buffer[i] = content_buffer[i] as char;
            }
            self.write_to_window(2, window_content_buffer)
        }
        else if self.in_use == 3{
            self.quad_f3.is_being_edited = true;
            let u8_name =self.filesystem.list_directory().unwrap().1[self.quad_f3.current_highlighted];
            self.quad_f3.current_file = u8_name;
            self.editing = true;
            let str_name = core::str::from_utf8(&u8_name).unwrap();
            let fd = self.filesystem.open_read(str_name).unwrap();
            let mut content_buffer = ['\0' as u8;MAX_FILE_BYTES];
            let mut window_content_buffer = ['\0'; (WINDOW_WIDTH-2) * (WINDOW_HEIGHT-2)];
            let contents = self.filesystem.read(fd, &mut content_buffer).unwrap();
            self.quad_f3.all_contents_u8 = content_buffer;
            self.quad_f3.find_u8_index();
            self.filesystem.close(fd);
            for i in 0..((WINDOW_WIDTH-2) * (WINDOW_HEIGHT-2)){
                window_content_buffer[i] = content_buffer[i] as char;
            }
            self.write_to_window(3, window_content_buffer)
        }
        else if self.in_use == 4{
            self.quad_f4.is_being_edited = true;
            let u8_name =self.filesystem.list_directory().unwrap().1[self.quad_f4.current_highlighted];
            self.quad_f4.current_file = u8_name;
            self.editing = true;
            let str_name = core::str::from_utf8(&u8_name).unwrap();
            let fd = self.filesystem.open_read(str_name).unwrap();
            let mut content_buffer = ['\0' as u8;MAX_FILE_BYTES];
            let mut window_content_buffer = ['\0'; (WINDOW_WIDTH-2) * (WINDOW_HEIGHT-2)];
            let contents = self.filesystem.read(fd, &mut content_buffer).unwrap();
            self.quad_f4.all_contents_u8 = content_buffer;
            self.quad_f4.find_u8_index();
            self.filesystem.close(fd);
            for i in 0..((WINDOW_WIDTH-2) * (WINDOW_HEIGHT-2)){
                window_content_buffer[i] = content_buffer[i] as char;
            }
            self.write_to_window(4, window_content_buffer)
        }
    }

    pub fn run_file(&mut self){

        if self.in_use == 1 && !self.bool_f1.0{
            
            self.quad_f1.is_being_edited = true;
            self.quad_f1.reset_colors();
            let u8_name =self.filesystem.list_directory().unwrap().1[self.quad_f1.current_highlighted];
            self.quad_f1.current_file = u8_name;
            let str_name = core::str::from_utf8(&u8_name).unwrap();
            let fd = self.filesystem.open_read(str_name).unwrap();
            let mut content_buffer = ['\0' as u8;MAX_FILE_BYTES];
            let contents = self.filesystem.read(fd, &mut content_buffer).unwrap();
            self.filesystem.close(fd);
            let program_text = core::str::from_utf8(&content_buffer[0..contents]).unwrap();
            self.int_f1 = Interpreter::new(program_text);
            
            self.bool_f1.0 = true;
            let empty = ['\0';(WINDOW_WIDTH-2)*(WINDOW_HEIGHT-2)];
            
            self.write_to_window(1, empty);
            self.process_ran +=1;
            
        }
        else if self.in_use == 2 && !self.bool_f2.0{
            
            let u8_name =self.filesystem.list_directory().unwrap().1[self.quad_f2.current_highlighted];
            self.quad_f2.current_file = u8_name;
            let str_name = core::str::from_utf8(&u8_name).unwrap();
            let fd = self.filesystem.open_read(str_name).unwrap();
            let mut content_buffer = ['\0' as u8;MAX_FILE_BYTES];
            let contents = self.filesystem.read(fd, &mut content_buffer).unwrap();
            self.filesystem.close(fd);
            let program_text = core::str::from_utf8(&content_buffer[0..contents]).unwrap();
            self.int_f2 = Interpreter::new(program_text);
            self.bool_f2.0 = true;
            let empty = ['\0';(WINDOW_WIDTH-2)*(WINDOW_HEIGHT-2)];
            self.write_to_window(2, empty);
        }
        else if self.in_use == 3 && !self.bool_f3.0{
            
            let u8_name =self.filesystem.list_directory().unwrap().1[self.quad_f3.current_highlighted];
            self.quad_f3.current_file = u8_name;
            let str_name = core::str::from_utf8(&u8_name).unwrap();
            let fd = self.filesystem.open_read(str_name).unwrap();
            let mut content_buffer = ['\0' as u8;MAX_FILE_BYTES];
            let contents = self.filesystem.read(fd, &mut content_buffer).unwrap();
            self.filesystem.close(fd);
            let program_text = core::str::from_utf8(&content_buffer[0..contents]).unwrap();
            self.int_f3 = Interpreter::new(program_text);
            self.bool_f3.0 = true;
            let empty = ['\0';(WINDOW_WIDTH-2)*(WINDOW_HEIGHT-2)];
            self.write_to_window(3, empty);
        }

     }

     pub fn run_one_instruction(&mut self) {
        
        if self.bool_f1.0 && self.turn_index==0{
            if (!self.bool_f1.1){
                self.running = true;
                //input stuff here
                if self.input_flag1{
                    self.int_f1.provide_input(&self.run_input[0..self.ri_index]);
                    self.ri_index = 0;
                    self.input_flag1 = false;
                }
                self.quad_f1.reset_colors();
                self.draw();
                let mut output = KernelOutput::new(self.quad_f1, 1);
                
                
                
                
                let result: TickResult<()> = self.int_f1.tick(&mut output);
                self.quad_f1 = output.window;
                self.quad_f1.update_contents(self.quad_f1.contents);
                // println!("{:?}",output.window.contents);
                // panic!();
                self.draw(); //maybe just update screen
                //println!()
                self.num_ticks[self.turn_index] +=1;
                match result{
                    TickResult::Ok(_) => {},
                    TickResult::Finished => {
                        let temp_in_use = self.in_use;
                        self.in_use = 1;
                        for i in "[DONE]".chars(){
                            self.edit_file_text(i);
                        }
                        self.in_use = temp_in_use;
                        self.draw();
                        self.bool_f1 = (false, false);
                        self.running = false;
                    },
                    TickResult::AwaitInput => {
                        self.waiting = true;
                        self.bool_f1.1 = true;
                    },
                    TickResult::Err(e) => {
                        println!("{:?}", e);
                        panic!();
                    },
                }
            }
        }
        // else{
        //     self.turn_index +=1;
        // }
        if self.bool_f2.0&& self.turn_index==1{
            if (!self.bool_f2.1){
                self.running = true;
                //input stuff here
                if self.input_flag2{
                    self.int_f2.provide_input(&self.run_input[0..self.ri_index]);
                    self.ri_index = 0;
                    self.input_flag2 = false;
                }
                self.quad_f2.reset_colors();
                self.draw();
                let mut output = KernelOutput::new(self.quad_f2, 2);
                
                
                
                
                let result: TickResult<()> = self.int_f2.tick(&mut output);
                self.quad_f2 = output.window;
                self.quad_f2.update_contents(self.quad_f2.contents);
                // println!("{:?}",output.window.contents);
                // panic!();
                self.draw(); //maybe just update screen
                self.num_ticks[self.turn_index] +=1;
                match result{
                    TickResult::Ok(_) => {},
                    TickResult::Finished => {
                        let temp_in_use = self.in_use;
                        self.in_use = 2;
                        for i in "[DONE]".chars(){
                            self.edit_file_text(i);
                        }
                        self.in_use = temp_in_use;
                        self.draw();
                        self.bool_f2 = (false, false);
                        self.running = false;
                    },
                    TickResult::AwaitInput => {
                        self.waiting = true;
                        self.bool_f2.1 = true;
                    },
                    TickResult::Err(e) => {
                        println!("{:?}", e);
                        panic!();
                    },
                }
            }
        }
        // else{
        //     self.turn_index+=1;
        // }
        if self.bool_f3.0&& self.turn_index==2{
            if (!self.bool_f3.1){
                self.running = true;
                //input stuff here
                if self.input_flag3{
                    self.int_f3.provide_input(&self.run_input[0..self.ri_index]);
                    self.ri_index = 0;
                    self.input_flag3 = false;
                }
                self.quad_f3.reset_colors();
                self.draw();
                let mut output = KernelOutput::new(self.quad_f3, 3);
                
                
                
                
                let result: TickResult<()> = self.int_f3.tick(&mut output);
                self.quad_f3 = output.window;
                self.quad_f3.update_contents(self.quad_f3.contents);
                // println!("{:?}",output.window.contents);
                // panic!();
                self.draw(); //maybe just update screen
                self.num_ticks[self.turn_index] +=1;
                match result{
                    TickResult::Ok(_) => {},
                    TickResult::Finished => {
                        let temp_in_use = self.in_use;
                        self.in_use = 3;
                        for i in "[DONE]".chars(){
                            self.edit_file_text(i);
                        }
                        self.in_use = temp_in_use;
                        self.draw();
                        self.bool_f3 = (false, false);
                        self.running = false;
                    },
                    TickResult::AwaitInput => {
                        self.waiting = true;
                        self.bool_f3.1 = true;
                    },
                    TickResult::Err(e) => {
                        println!("{:?}", e);
                        panic!();
                    },
                }
            }
        }
        self.turn_index +=1;
        self.turn_index = self.turn_index %3;
     }


    pub fn update_borders(&mut self){
        self.quad_f1.update_borders();
        self.quad_f2.update_borders();
        self.quad_f3.update_borders();
        self.quad_f4.update_borders();

    }
    pub fn update_screen(&mut self){
        for i in 0..BUFFER_HEIGHT-1{
            for j in 0..BUFFER_WIDTH{
                if i == 0 && j < BUFFER_WIDTH-TASK_MANAGER_WIDTH{
                    if j < MAX_USER_INPUT_BYTES{
                        self.screen[i][j] = self.user_input[j];
                    }
                    else{
                        self.screen[i][j] = '\0';
                    }
                    
                }
                else if j >= BUFFER_WIDTH-TASK_MANAGER_WIDTH{
                    self.screen[i][j] = self.task_manager[i][j-(BUFFER_WIDTH-TASK_MANAGER_WIDTH)];
                }
                else{
                    if (i <= WINDOW_HEIGHT && j < WINDOW_WIDTH){
                        
                        self.screen[i][j] = self.quad_f1.vga[(i-1)%WINDOW_HEIGHT][j%WINDOW_WIDTH];
                        self.foreground[i][j] = self.quad_f1.foreground[(i-1)%WINDOW_HEIGHT][j%WINDOWS_WIDTH];
                        self.background[i][j] = self.quad_f1.background[(i-1)%WINDOW_HEIGHT][j%WINDOWS_WIDTH];
                        
                    }
                    if (i <= WINDOW_HEIGHT && j >= WINDOW_WIDTH){
                        self.screen[i][j-1] = self.quad_f2.vga[(i-1)%WINDOW_HEIGHT][j%WINDOW_WIDTH];
                        self.foreground[i][j-1] = self.quad_f2.foreground[(i-1)%WINDOW_HEIGHT][j%WINDOW_WIDTH];
                        self.background[i][j-1] = self.quad_f2.background[(i-1)%WINDOW_HEIGHT][j%WINDOW_WIDTH];

                        if self.in_use==1 && j%WINDOW_WIDTH==0 && self.screen[i][j-1] == '.' {
                            self.screen[i][j-1] = '*';
                        }
                    }
                    if (i >= WINDOW_HEIGHT && j < WINDOW_WIDTH){
                        self.screen[i][j] = self.quad_f3.vga[i%WINDOW_HEIGHT][j%WINDOW_WIDTH];
                        self.foreground[i][j] = self.quad_f3.foreground[i%WINDOW_HEIGHT][j%WINDOW_WIDTH];
                        self.background[i][j] = self.quad_f3.background[i%WINDOW_HEIGHT][j%WINDOW_WIDTH];
                        if self.in_use==1 && i%WINDOW_HEIGHT==0 && self.screen[i][j] == '.' {
                            self.screen[i][j] = '*';
                        }
                        //add stuff for * if top selected
                    }
                    if (i >= WINDOW_HEIGHT && j >= WINDOW_WIDTH){
                        self.screen[i][j-1] = self.quad_f4.vga[(i)%WINDOW_HEIGHT][j%WINDOW_WIDTH];
                        self.foreground[i][j-1] = self.quad_f4.foreground[(i)%WINDOW_HEIGHT][j%WINDOW_WIDTH];
                        self.background[i][j-1] = self.quad_f4.background[(i)%WINDOW_HEIGHT][j%WINDOW_WIDTH];
                        if self.in_use==2 && i%WINDOW_HEIGHT==0 && self.screen[i][j-1] == '.' {
                            self.screen[i][j-1] = '*';
                        }
                        if self.in_use==3 && j%WINDOW_WIDTH==0 && self.screen[i][j-1] == '.' {
                            self.screen[i][j-1] = '*';
                        }
                        if self.in_use==1 && i%WINDOW_HEIGHT==0 && j%WINDOW_WIDTH == 0 && self.screen[i][j-1] == '.' {
                            self.screen[i][j-1] = '*';
                        }
                        
                        //add stuff for * if top selected
                    }
                }

            }
        }
    }

    pub fn update_colors(&mut self){
        self.quad_f1.update_colors();
        self.quad_f2.update_colors();
        self.quad_f3.update_colors();
        self.quad_f4.update_colors();

    }
    pub fn draw(&mut self) {

        self.update_colors();
        self.update_borders();
        self.update_screen();
        
        for i in 0..BUFFER_HEIGHT{
            for j in 0..BUFFER_WIDTH{
                plot(self.screen[i][j], j, i, ColorCode::new(self.foreground[i][j], self.background[i][j]));
            }
        }
        
    }

    pub fn tick_numbers(&mut self, spot: usize) -> (char, char, char, char){
        let mut spot1 = '0';
        let mut spot2 = '0';
        let mut spot3 = '0';
        let mut spot4 = '0';
        let mut count = 4;


        spot4 = char::from_digit((self.num_ticks[spot] % 10) as u32, 10).unwrap();
        spot3 =  char::from_digit((self.num_ticks[spot] / 10 % 10)as u32, 10).unwrap();
        spot2 = char::from_digit((self.num_ticks[spot]/ 10 / 10 % 10) as u32, 10).unwrap();
        spot1 = char::from_digit((self.num_ticks[spot]/ 10 / 10 / 10 % 10) as u32, 10).unwrap();

        let mut spots = [spot1, spot2, spot3, spot4];
        for i in spots{
            if i != '0' || count ==1{
                break;
            }
            else{
                count -=1;
            }
        }
        if count == 3 {
            spot1 = spot2;
            spot2 = spot3;
            spot3 = spot4;
            spot4 = ' ';
        } else if count == 2 {
            spot1 = spot3;
            spot2 = spot4;
            spot3 = ' ';
            spot4 = ' ';
        } else if count == 1 {
            spot1 = spot4;
            spot2 = ' ';
            spot3 = ' ';
            spot4 = ' ';
        }
        return (spot1, spot2, spot3, spot4)



    }
    pub fn draw_proc_status(&mut self) {
        self.task_manager[0][0] = 'F';
        self.task_manager[0][1] = '1';
        self.task_manager[2][0] = 'F';
        self.task_manager[2][1] = '2';
        self.task_manager[4][0] = 'F';
        self.task_manager[4][1] = '3';
        self.task_manager[6][0] = 'F';
        self.task_manager[6][1] = '4';
        let f1_ticks = self.tick_numbers(0);
        let f2_ticks = self.tick_numbers(1);
        let f3_ticks = self.tick_numbers(2);
        let f4_ticks = self.tick_numbers(3);
        // for i in 0..4{

        //     self.task_manager[1][i] = f1_ticks.i;
        // }
        self.task_manager[1][0] = f1_ticks.0;
        self.task_manager[1][1] = f1_ticks.1;
        self.task_manager[1][2] = f1_ticks.2;
        self.task_manager[1][3] = f1_ticks.3;

        self.task_manager[3][0] = f2_ticks.0;
        self.task_manager[3][1] = f2_ticks.1;
        self.task_manager[3][2] = f2_ticks.2;
        self.task_manager[3][3] = f2_ticks.3;

        self.task_manager[5][0] = f3_ticks.0;
        self.task_manager[5][1] = f3_ticks.1;
        self.task_manager[5][2] = f3_ticks.2;
        self.task_manager[5][3] = f3_ticks.3;

        self.task_manager[7][0] = f4_ticks.0;
        self.task_manager[7][1] = f4_ticks.1;
        self.task_manager[7][2] = f4_ticks.2;
        self.task_manager[7][3] = f4_ticks.3;
        self.update_screen()
    }

    
}


pub struct KernelOutput{
    window: Window,
    which_one: usize,
    
}

impl KernelOutput {
    fn new(window: Window, which_one: usize) -> Self{
        let mut window = window;
        let mut which_one = which_one;
        Self { window: window, which_one: which_one,  }
    }


    pub fn write_to_window(&mut self, window_num: usize, contents: [char; (WINDOW_WIDTH-2)*(WINDOW_HEIGHT-2)]){


        let mut char_count = 0;
        let mut row_count = 0;
        let mut c = [['\0';WINDOW_WIDTH-2];WINDOW_HEIGHT-2];
        for i in contents{
            if i as char =='\n'{
                
                char_count = 0;
                row_count +=1;
                continue;
            }
            let mut val = 2;
            if char_count >= WINDOW_WIDTH-val{
                char_count = 0;
                row_count +=1;
            }
            if row_count >=WINDOW_HEIGHT-2{
                break;
            }
            c[row_count][char_count] = i as char;
            char_count+=1;
            
            
        }


        self.window.update_contents(c);
        self.window.find_contents_index();
        
        //self.update_screen();
        //println!("{:?}", contents);
    }
    
}

impl InterpreterOutput for KernelOutput {
    fn print(&mut self, chars: &[u8]) {
        // println!("{:?}",chars);
        // panic!();
        //let mut buf = ['\0'; (WINDOW_WIDTH-2)*(WINDOW_HEIGHT-2)];
        // if self.window.contents[0][0] != '\0'{
        //     
        // }
        
        for (i,val) in chars.iter().enumerate(){
            self.window.type_char(*val as char);
        }
        self.window.edit_press_enter();

        //self.write_to_window(self.which_one, buf);
    }
}



fn text_color() -> ColorCode {
    ColorCode::new(Color::White, Color::Black)
}

fn highlight_color() -> ColorCode {
    ColorCode::new(Color::Black, Color::White)
}