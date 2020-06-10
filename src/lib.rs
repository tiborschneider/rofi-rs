// Copyright 2020 Tibor Schneider
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Rofi ui manager
//! Spawn rofi windows, and parse the result appropriately.
//!
//! ## Simple example
//!
//! ```
//! use rofi;
//! use std::{fs, env};
//!
//! let dir_entries = fs::read_dir(env::current_dir().unwrap())
//!     .unwrap()
//!     .map(|d| format!("{:?}", d.unwrap().path()))
//!     .collect::<Vec<String>>();
//!
//! match rofi::Rofi::new(&dir_entries).run() {
//!     Ok(choice) => println!("Choice: {}", choice),
//!     Err(rofi::Error::Interrupted) => println!("Interrupted"),
//!     Err(e) => println!("Error: {}", e)
//! }
//! ```
//!
//! ## Example of returning an index
//! `rofi` can also be used to return an index of the selected item:
//!
//! ```
//! use rofi;
//! use std::{fs, env};
//!
//! let dir_entries = fs::read_dir(env::current_dir().unwrap())
//!     .unwrap()
//!     .map(|d| format!("{:?}", d.unwrap().path()))
//!     .collect::<Vec<String>>();
//!
//! match rofi::Rofi::new(&dir_entries).run_index() {
//!     Ok(element) => println!("Choice: {}", element),
//!     Err(rofi::Error::Interrupted) => println!("Interrupted"),
//!     Err(rofi::Error::NotFound) => println!("User input was not found"),
//!     Err(e) => println!("Error: {}", e)
//! }
//! ```
//!
//! ## Example of using pango formatted strings
//! `rofi` can display pango format. Here is a simple example (you have to call
//! the `self..pango` function).
//!
//! ```
//! use rofi;
//! use rofi::pango::{Pango, FontSize};
//! use std::{fs, env};
//!
//! let entries: Vec<String> = vec![
//!     Pango::new("Option 1").size(FontSize::Small).fg_color("#666000").build(),
//!     Pango::new("Option 2").size(FontSize::Large).fg_color("#deadbe").build(),
//! ];
//!
//! match rofi::Rofi::new(&entries).pango().run() {
//!     Ok(element) => println!("Choice: {}", element),
//!     Err(rofi::Error::Interrupted) => println!("Interrupted"),
//!     Err(e) => println!("Error: {}", e)
//! }
//! ```

#![deny(missing_docs, missing_debug_implementations, rust_2018_idioms)]

pub mod pango;

use std::process::{Command, Stdio, Child};
use thiserror::Error;
use std::io::{Read, Write};

/// # Rofi Window Builder
/// Rofi struct for displaying user interfaces. This struct is build after the
/// non-consuming builder pattern. You can prepare a window, and draw it
/// multiple times without reconstruction and reallocation. You can choose to
/// return a handle to the child process `RofiChild`, which allows you to kill
/// the process.
#[derive(Debug)]
pub struct Rofi<'a, T>
where
    T: AsRef<str>
{
    elements: &'a Vec<T>,
    case_sensitive: bool,
    lines: Option<usize>,
    width: Width,
    format: Format,
    args: Vec<String>,
}

/// Rofi child process.
#[derive(Debug)]
pub struct RofiChild<T> {
    num_elements: T,
    p: Child,
}

impl<T> RofiChild<T> {
    fn new(p: Child, arg: T) -> Self {
        Self{num_elements: arg, p}
    }
    /// Kill the Rofi process
    pub fn kill(&mut self) -> Result<(), Error> {
        Ok(self.p.kill()?)
    }
}

impl RofiChild<String> {
    /// Wait for the result and return the output as a String.
    fn wait_with_output(&mut self) -> Result<String, Error> {
        let status = self.p.wait()?;
        if status.success() {
            let mut buffer = String::new();
            if let Some(mut reader) = self.p.stdout.take() {
                reader.read_to_string(&mut buffer)?;
            }
            if buffer.ends_with('\n') {
                buffer.pop();
            }
            if buffer.len() == 0 {
                Err(Error::Blank{})
            } else {
                Ok(buffer)
            }
        } else {
            Err(Error::Interrupted{})
        }
    }
}

impl RofiChild<usize> {
    /// Wait for the result and return the output as an usize.
    fn wait_with_output(&mut self) -> Result<usize, Error> {
        let status = self.p.wait()?;
        if status.success() {
            let mut buffer = String::new();
            if let Some(mut reader) = self.p.stdout.take() {
                reader.read_to_string(&mut buffer)?;
            }
            if buffer.ends_with('\n') {
                buffer.pop();
            }
            if buffer.len() == 0 {
                Err(Error::Blank{})
            } else {
                let idx: isize = buffer.parse::<isize>()?;
                if idx < 0 || idx > self.num_elements as isize {
                    Err(Error::NotFound{})
                } else {
                    Ok(idx as usize)
                }
            }
        } else {
            Err(Error::Interrupted{})
        }
    }
}

impl<'a, T> Rofi<'a, T>
where
    T: AsRef<str>
{
    /// Generate a new, unconfigured Rofi window based on the elements provided.
    pub fn new(elements: &'a Vec<T>) -> Self {
        Self {
            elements,
            case_sensitive: false,
            lines: None,
            width: Width::None,
            format: Format::Text,
            args: Vec::new()
        }
    }

    /// Show the window, and return the selected string, including pango
    /// formatting if available
    pub fn run(&self) -> Result<String, Error> {
        self.spawn()?.wait_with_output()
    }

    /// show the window, and return the index of the selected string This
    /// function will overwrite any subsequent calls to `self.format`.
    pub fn run_index(&mut self) -> Result<usize, Error> {
        self.spawn_index()?.wait_with_output()
    }

    /// enable pango markup
    pub fn pango(&mut self) -> &mut Self {
        self.args.push("-markup-rows".to_string());
        self
    }

    /// enable password mode
    pub fn password(&mut self) -> &mut Self {
        self.args.push("-password".to_string());
        self
    }

    /// Sets the number of lines.
    /// If this funciton is not called, use the number of lines provided in the
    /// elements vector.
    pub fn lines(&mut self, l: usize) -> &mut Self {
        self.lines = Some(l);
        self
    }

    /// Set the width of the window (overwrite the theme settings)
    pub fn width(&mut self, w: Width) -> Result<&mut Self, Error> {
        w.check()?;
        self.width = w;
        Ok(self)
    }

    /// Sets the case sensitivity (disabled by default)
    pub fn case_sensitive(&mut self, sensitivity: bool) -> &mut Self {
        self.case_sensitive = sensitivity;
        self
    }

    /// Set the prompt of the rofi window
    pub fn prompt(&mut self, prompt: impl Into<String>) -> &mut Self {
        self.args.push("-p".to_string());
        self.args.push(prompt.into());
        self
    }

    /// Set the rofi theme
    /// This will make sure that rofi uses `~/.config/rofi/{theme}.rasi`
    pub fn theme(&mut self, theme: Option<impl Into<String>>) -> &mut Self {
        if let Some(t) = theme {
            self.args.push("-theme".to_string());
            self.args.push(t.into());
        }
        self
    }

    /// Set the return format of the rofi call. Default is `Format::Text`. If
    /// you call `self.spawn_index` later, the format will be overwritten with
    /// `Format::Index`.
    pub fn return_format(&mut self, format: Format) -> &mut Self {
        self.format = format;
        self
    }

    /// Returns a child process with the pre-prepared rofi window
    /// The child will produce the exact output as provided in the elements vector.
    pub fn spawn(&self) -> Result<RofiChild<String>, std::io::Error> {
        Ok(RofiChild::new(self.spawn_child()?, String::new()))
    }

    /// Returns a child process with the pre-prepared rofi window.
    /// The child will produce the index of the chosen element in the vector.
    /// This function will overwrite any subsequent calls to `self.format`.
    pub fn spawn_index(&mut self) -> Result<RofiChild<usize>, std::io::Error> {
        self.format = Format::Index;
        Ok(RofiChild::new(self.spawn_child()?, self.elements.len()))
    }

    fn spawn_child(&self) -> Result<Child, std::io::Error> {
        let mut child = Command::new("rofi")
            .arg("-dmenu")
            .args(&self.args)
            .arg("-format")
            .arg(self.format.as_arg())
            .arg("-lines")
            .arg(match self.lines.as_ref() {
                Some(s) => format!("{}", s),
                None => format!("{}", self.elements.len())
            })
            .arg(match self.case_sensitive {
                true => "-case-sensitive",
                false => "-i"
            })
            .args(match self.width {
                Width::None => vec![],
                Width::Percentage(x) => vec!["-width".to_string(), format!("{}", x)],
                Width::Pixels(x) => vec!["-width".to_string(), format!("{}", x)],
                Width::Characters(x) => vec!["-width".to_string(), format!("-{}", x)],
            })
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        if let Some(mut writer) = child.stdin.take() {
            for element in self.elements {
                writer.write_all(element.as_ref().as_bytes())?;
                writer.write(b"\n")?;
            }
        }
        Ok(child)
    }

}

/// Width of the rofi window to overwrite the default width from the rogi theme.
#[derive(Debug)]
pub enum Width {
    /// No width specified, use the default one from the theme
    None,
    /// Width in percentage of the screen, must be between 0 and 100
    Percentage(usize),
    /// Width in pixels, must be greater than 100
    Pixels(usize),
    /// Estimates the width based on the number of characters.
    Characters(usize)
}

impl Width {
    fn check(&self) -> Result<(), Error> {
        match self {
            Self::Percentage(x) => {
                if *x > 100 {Err(Error::InvalidWidth("Percentage must be between 0 and 100"))} else {Ok(())}
            },
            Self::Pixels(x) => {
                if *x <= 100 {Err(Error::InvalidWidth("Pixels must be larger than 100"))} else {Ok(())}
            }
            _ => Ok(())
        }
    }
}

/// Different modes, how rofi should return the results
#[derive(Debug)]
pub enum Format {
    /// Regular text, including markup
    #[allow(dead_code)]
    Text,
    /// Text, where the markup is removed
    StrippedText,
    /// Text with the exact user input
    UserInput,
    /// Index of the chosen element
    Index
}

impl Format {
    fn as_arg(&self) -> &'static str {
        match self {
            Format::Text => "s",
            Format::StrippedText => "p",
            Format::UserInput => "f",
            Format::Index => "i",
        }
    }
}

/// Rofi Error Type
#[derive(Error, Debug)]
pub enum Error {
    /// IO Error
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    /// Parse Int Error, only occurs when getting the index.
    #[error("Parse Int Error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    /// Error returned when the user has interrupted the action
    #[error("User interrupted the action")]
    Interrupted,
    /// Error returned when the user chose a blank option
    #[error("User chose a blank line")]
    Blank,
    /// Error returned the width is invalid, only returned in Rofi::width()
    #[error("Invalid width: {0}")]
    InvalidWidth(&'static str),
    /// Error, when the input of the user is not found. This only occurs when
    /// getting the index.
    #[error("User input was not found")]
    NotFound
}

#[cfg(test)]
mod rofitest {
    use super::*;
    #[test]
    fn simple_test() {
        let options = vec!["a", "b", "c", "d"];
        let empty_options: Vec<String> = Vec::new();
        match Rofi::new(&options).prompt("choose c").run() {
            Ok(ret) => assert!(ret == "c"),
            _ => assert!(false)
        }
        match Rofi::new(&options).prompt("chose c").run_index() {
            Ok(ret) => assert!(ret == 2),
            _ => assert!(false)
        }
        match Rofi::new(&options)
            .prompt("press escape")
            .width(Width::Percentage(15)).unwrap()
            .run_index() {
            Err(Error::Interrupted) => assert!(true),
            _ => assert!(false)
        }
        match Rofi::new(&options).prompt("Enter something wrong").run_index() {
            Err(Error::NotFound) => assert!(true),
            _ => assert!(false)
        }
        match Rofi::new(&empty_options)
            .prompt("Enter password")
            .password()
            .return_format(Format::UserInput)
            .run() {
            Ok(ret) => assert!(ret == "password"),
            _ => assert!(false)
        }
    }
}
