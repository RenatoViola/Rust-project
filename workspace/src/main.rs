use rand::Rng;
use rand_distr::num_traits::ToPrimitive;
use rand_distr::{Distribution, Normal};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct DailyStats {
    portions_sold: usize,
    portions_left: usize,
    worker_stats: Vec<usize>, // each index in the vector represents the portions sold for each worker
}

struct Bakery {
    stats: Vec<DailyStats>,
}

impl Bakery {
    fn operate(
        &self,
        is_open: Arc<Mutex<bool>>,
        regular_queue: Arc<Mutex<VecDeque<Client>>>,
        priority_queue: Arc<Mutex<VecDeque<Client>>>,
        cakes: Arc<Mutex<VecDeque<usize>>>,
    ) {
        let mut worker_handles = vec![];

        // Launch 6 worker threads
        for worker_id in 0..6 {
            let is_open_clone = Arc::clone(&is_open);
            let regular_queue_clone = Arc::clone(&regular_queue);
            let priority_queue_clone = Arc::clone(&priority_queue);
            let cakes_clone = Arc::clone(&cakes);

            let handle = thread::spawn(move || {
                let mut portions_sold = 0;

                while *is_open_clone.lock().unwrap() {
                    portions_sold += 1;
                    thread::sleep(Duration::from_millis(500));
                }

                (worker_id, portions_sold)
            });

            worker_handles.push(handle);
        }

        // Collect results from each worker thread
        let mut worker_stats = vec![0; 6]; // Initialize a Vec of size 6 with all elements set to 0
        for handle in worker_handles {
            let (id, portions_sold) = handle.join().unwrap();
            worker_stats[id] = portions_sold;
        }

        for (id, sold) in worker_stats.iter().enumerate() {
            println!("Worker with id {} sold {} portions!", id, sold);
        }
    }
}

#[derive(Clone, Copy)]
struct Client {
    id: u32,
    is_priority: bool,
    portions_ordered: usize,
}

impl Client {
    fn new(id: u32) -> Self {
        let mut rng = rand::thread_rng();
        let is_priority = rng.gen_bool(0.2);
        let portions_ordered = rng.gen_range(1..=18);
        Self {
            id,
            is_priority,
            portions_ordered,
        }
    }
}

#[derive(Clone, Copy)]
struct Worker {
    id: u32,
    is_priority: bool,
    portions_sold: usize,
    client: Option<Client>,
}

impl Worker {
    fn new(id: u32, is_priority: bool) -> Self {
        Self {
            id,
            is_priority,
            portions_sold: 0,
            client: None,
        }
    }
}

fn cake_production(is_morning: Arc<Mutex<bool>>, cakes: Arc<Mutex<VecDeque<usize>>>) {
    loop {
        // Check if it's morning
        if *is_morning.lock().unwrap() {
            // Simulate producing one cake with a delay
            thread::sleep(Duration::from_millis(500));
            let n_cakes = {
                let mut cakes = cakes.lock().unwrap();
                cakes.push_back(6); // Each cake starts with 6 portions
                cakes.len()
            };
            println!("Baked a cake. Currently have {} cakes in stack.", n_cakes);
        } else {
            // Afternoon production: reset with a fixed number of cakes
            let n_cakes = {
                let mut cakes = cakes.lock().unwrap();
                *cakes = VecDeque::from(vec![6; 50]); // Afternoon stack of 50 cakes
                cakes.len()
            };
            println!(
                "It is the afternoon. Stack has {} cakes available.",
                n_cakes
            );
            break; // Exit loop once afternoon stack is set
        }
    }
}

fn customer_arrival(
    is_open: Arc<Mutex<bool>>,
    regular_queue: Arc<Mutex<VecDeque<Client>>>,
    priority_queue: Arc<Mutex<VecDeque<Client>>>,
) {
    let mut id = 0;

    loop {
        if *is_open.lock().unwrap() {
            let client = Client::new(id);
            {
                let mut queue = if client.is_priority {
                    priority_queue.lock().unwrap()
                } else {
                    regular_queue.lock().unwrap()
                };
                queue.push_back(client);
                println!(
                    "Customer with id {} and priority {} arrived. | Size of corresponding queue: {}",
                    id,
                    client.is_priority,
                    queue.len()
                );
            }
            id += 1;

            // Simulate a short delay to avoid a busy loop
            let normal = Normal::new(2.0, 0.5).unwrap();
            let arrival_time = (normal.sample(&mut rand::thread_rng())).to_u64().unwrap();
            thread::sleep(Duration::from_secs(arrival_time));
        } else {
            println!("Day has ended, no more customers.");
            break;
        }
    }
}

fn main() {
    let mut bakery = Bakery { stats: Vec::new() };

    // Shared state to control bakery's open/close status
    let num_days = 1;
    for day in 1..=num_days {
        let regular_queue = Arc::new(Mutex::new(VecDeque::<Client>::new()));
        let priority_queue = Arc::new(Mutex::new(VecDeque::<Client>::new()));
        let cakes = Arc::new(Mutex::new(VecDeque::<usize>::with_capacity(50)));

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

        let cakes_clone = Arc::clone(&cakes);
        let production_handle = thread::spawn(move || {
            cake_production(is_morning, cakes_clone);
        });

        let is_open_clone = Arc::clone(&is_open);
        let regular_queue_clone = Arc::clone(&regular_queue);
        let priority_queue_clone = Arc::clone(&priority_queue);
        let customer_handle = thread::spawn(move || {
            customer_arrival(is_open_clone, regular_queue_clone, priority_queue_clone);
        });

        // Start the bakery thread for the day
        bakery.operate(is_open, regular_queue, priority_queue, cakes);

        production_handle.join().unwrap();
        customer_handle.join().unwrap();

        // Small interval to simulate break between days
        thread::sleep(Duration::from_secs(5));
    }
}
