# Rofi Library for Rust
Spawn [rofi](https://github.com/davatorium/rofi) windows, and parse the result appropriately.

## Simple example

```rust
use rofi;
use std::{fs, env};

let dir_entries = fs::read_dir(env::current_dir().unwrap())
    .unwrap()
    .map(|d| format!("{:?}", d.unwrap().path()))
    .collect::<Vec<String>>();

match rofi::Rofi::new(&dir_entries).run() {
    Ok(choice) => println!("Choice: {}", choice),
    Err(rofi::Error::Interrupted) => println!("Interrupted"),
    Err(e) => println!("Error: {}", e)
}
```

## Example of returning an index
`rofi` can also be used to return an index of the selected item:

```rust
use rofi;
use std::{fs, env};

let dir_entries = fs::read_dir(env::current_dir().unwrap())
    .unwrap()
    .map(|d| format!("{:?}", d.unwrap().path()))
    .collect::<Vec<String>>();

match rofi::Rofi::new(&dir_entries).run_index() {
    Ok(element) => println!("Choice: {}", element),
    Err(rofi::Error::Interrupted) => println!("Interrupted"),
    Err(rofi::Error::NotFound) => println!("User input was not found"),
    Err(e) => println!("Error: {}", e)
}
```

## Example of using pango formatted strings
`rofi` can display pango format. Here is a simple example (you have to call
the `self..pango` function).

```rust
use rofi;
use rofi::pango::{Pango, FontSize};
use std::{fs, env};

let entries: Vec<String> = vec![
    Pango::new("Option 1").size(FontSize::Small).fg_color("#666000").build(),
    Pango::new("Option 2").size(FontSize::Large).fg_color("#deadbe").build(),
];

match rofi::Rofi::new(&entries).pango().run() {
    Ok(element) => println!("Choice: {}", element),
    Err(rofi::Error::Interrupted) => println!("Interrupted"),
    Err(e) => println!("Error: {}", e)
}
```

## Example of showing a message with no inputs
`rofi` can display a message without any inputs. This is commonly used for error reporting.

```rust
use rofi;

match rofi::Rofi::new_message("Something went wrong").run() {
    Err(rofi::Error::Blank) => () // the expected case
    Ok(_) => ()  // should not happen
    Err(_) => () // Something went wrong
}
```
