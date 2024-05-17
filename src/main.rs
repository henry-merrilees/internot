#![feature(duration_constructors)]
use chrono;
use std::process::Command;
use std::sync::{Arc, Condvar, Mutex};

struct State {
    disabling: bool,
    terminating: bool,
}

fn main() {
    let initial_state = State {
        disabling: false,
        terminating: false,
    };

    let shared_state = Arc::new((Mutex::new(initial_state), Condvar::new()));

    let shared_state_clone = shared_state.clone();
    std::thread::spawn(move || loop {
        let (lock, cvar) = &*shared_state_clone;
        let mut state = lock.lock().unwrap();
        while !state.disabling {
            state = cvar.wait(state).unwrap();
            if state.terminating {
                return;
            }
        }
        internet_off();
        drop(state); // drop as soon as possible
        std::thread::sleep(std::time::Duration::from_secs(1));
    });

    let shared_state_clone = shared_state.clone();
    ctrlc::set_handler(move || {
        let (lock, cvar) = &*shared_state_clone;
        let mut state = lock.lock().unwrap();
        state.disabling = false;
        state.terminating = true;
        cvar.notify_one();
        internet_on();
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    let (lock, cvar) = &*shared_state;
    loop {
        let mut state = lock.lock().unwrap();
        state.disabling = true;
        println!("Disabling internet.");
        drop(state);
        cvar.notify_one(); // TODO I don't think this is necessary
        let duration = get_duration();
        let mut state = lock.lock().unwrap();
        state.disabling = false;
        drop(state);
        internet_on();
        let done_time = chrono::Local::now() + duration;
        let done_time = done_time.format("%I:%M:%S %p");

        println!("Internet enabled until {}.", done_time);
        std::thread::sleep(duration);
    }
}

fn get_duration() -> std::time::Duration {
    println!("How long do you want to enable the internet for (minutes)?");
    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        match input.trim().parse::<u64>() {
            Ok(duration) => return std::time::Duration::from_mins(duration),
            Err(_) => {
                println!("Please enter a valid number");
                continue;
            }
        };
    }
}

fn internet_on() {
    Command::new("networksetup")
        .args(["-setairportpower", "en0", "on"])
        .output()
        .expect("Failed to enable internet");
}

fn internet_off() {
    Command::new("networksetup")
        .args(["-setairportpower", "en0", "off"])
        .output()
        .expect("Failed to disable internet");
}
