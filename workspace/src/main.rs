use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::collections::VecDeque;

struct Bakery {
    regular_queue: VecDeque<Client>,
    priority_queue: VecDeque<Client>,
    workers: Vec<Worker>,
    cake_stack: Vec<Cake>,
}

impl Bakery {
    fn operate(&mut self, is_open: Arc<Mutex<bool>>) {
        while *is_open.lock().unwrap() {
            // Simulate serving clients while the bakery is open
            println!("Bakery is operating.");
            
            // Additional logic for serving clients and managing cakes...
            
            // Simulate a short delay to avoid a busy loop
            thread::sleep(Duration::from_millis(1000));
        }
        println!("The bakery is now closed for the day.");
    }
}

struct Client {
    id: usize,
}

impl Client {
    fn new(id: usize) -> Self {
        Self { id }
    }
}

struct Worker {
    id: usize,
}

impl Worker {
    fn new(id: usize) -> Self {
        Self { id }
    }
}

#[derive(Clone, Copy)]
struct Cake {
    id: usize,
    remaining_portions: usize,
}

impl Cake {
    fn new(id: usize) -> Self {
        Self {
            id,
            remaining_portions: 6,
        }
    }
}

fn cake_production(is_morning: Arc<Mutex<bool>>) {
    while *is_morning.lock().unwrap() {
        println!("Making cakes continuously.");
        // Simulate a short delay to avoid a busy loop
        thread::sleep(Duration::from_millis(2000));
    }
    println!("Made 50 cakes and put them on stack.");
}

fn customer_arrival(is_open: Arc<Mutex<bool>>) {
    while *is_open.lock().unwrap() {
        println!("Customer arrived.");
        // Simulate a short delay to avoid a busy loop
        thread::sleep(Duration::from_millis(500));
    }
    println!("Day has ended, no more customers.");
}

fn main() {
    let mut bakery = Bakery {
        regular_queue: VecDeque::new(),
        priority_queue: VecDeque::new(),
        workers: vec![Worker::new(1)],
        cake_stack: vec![Cake::new(1); 50],
    };

    // Shared state to control bakery's open/close status
    for day in 1..=2 {

        println!("Starting day {}", day);
        
        // Reset the `open` flag at the beginning of each day
        let is_open = Arc::new(Mutex::new(true));
        let is_morning = Arc::new(Mutex::new(true));

        // Start the timer thread for "10 hours" (10 seconds for testing)
        let is_open_timer = Arc::clone(&is_open);
        let is_morning_timer = Arc::clone(&is_morning);
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(5)); // Simulate 5 hours with a shorter duration
            {
                let mut is_morning = is_morning_timer.lock().unwrap();
                *is_morning = false; // now it's afternoon
                println!("5 hours have passed. It is the afternoon for day {}.", day);
            }
            thread::sleep(Duration::from_secs(5)); // Simulate 5 hours with a shorter duration, for a total of 10 hours
            let mut is_open = is_open_timer.lock().unwrap();
            *is_open = false;
            println!("10 hours have passed. Closing the bakery for day {}.", day);
        });

        let production_handle = thread::spawn(move || {
            cake_production(is_morning);
        });

        let is_open_clone = Arc::clone(&is_open);
        let customer_handle = thread::spawn(move || {
            customer_arrival(is_open_clone);
        });

        // Start the bakery thread for the day
        bakery.operate(is_open);

        production_handle.join().unwrap();
        customer_handle.join().unwrap();

        // Small interval to simulate break between days
        thread::sleep(Duration::from_secs(2));
    }
}
