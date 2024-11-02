use rand::Rng;
use rand_distr::num_traits::ToPrimitive;
use rand_distr::{Distribution, Normal};
use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[allow(dead_code)]
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
        &mut self,
        is_open: Arc<Mutex<bool>>,
        regular_queue: Arc<Mutex<VecDeque<Client>>>,
        priority_queue: Arc<Mutex<VecDeque<Client>>>,
        cakes: Arc<Mutex<VecDeque<usize>>>,
    ) {
        let mut handles = vec![];

        // Launch 6 worker threads
        for id in 0..6 {
            let is_open_clone = Arc::clone(&is_open);
            let regular_queue_clone = Arc::clone(&regular_queue);
            let priority_queue_clone = Arc::clone(&priority_queue);
            let cakes_clone = Arc::clone(&cakes);

            let handle = thread::spawn(move || {
                let mut worker = Worker::new(id);
                worker.service(
                    is_open_clone,
                    regular_queue_clone,
                    priority_queue_clone,
                    cakes_clone,
                )
            });

            handles.push(handle);
        }

        self.gather_daily_stats(cakes, handles);
    }

    fn gather_daily_stats(
        &mut self,
        cakes: Arc<Mutex<VecDeque<usize>>>,
        handles: Vec<JoinHandle<(usize, usize)>>,
    ) {
        // Collect results from each worker thread
        let mut worker_stats = vec![0; 6]; // Initialize a Vec of size 6 with all elements set to 0
        let mut best_worker = 10; // arbitrary value for a worker id that exists
        let mut current_best_sales = 0;
        for handle in handles {
            let (id, portions_sold) = handle.join().unwrap();
            worker_stats[id] = portions_sold;

            if portions_sold > current_best_sales {
                best_worker = id;
                current_best_sales = portions_sold;
            }
        }

        let portions_sold: usize = worker_stats.iter().sum();

        let portions_left = {
            let cakes = cakes.lock().unwrap();
            cakes.iter().sum::<usize>()
        };

        self.stats.push(DailyStats {
            portions_sold,
            portions_left,
            worker_stats,
        });

        println!(
            "Portions sold for the day: {} | Portions left for the day: {}",
            portions_sold, portions_left
        );

        let divided_portions = portions_left / 6;
        let portions_remainder = portions_left % 6;
        if portions_remainder == 0 {
            println!(
                "{} was the best worker and everyone takes {} portions home today.",
                best_worker, divided_portions
            );
        } else {
            println!(
                "{} was the best worker, so he's taking {} portions home today. Everyone else takes {} portions of cake.",
                best_worker, (divided_portions + portions_remainder), divided_portions
            );
        }
    }

    fn general_stats(&self) {
        // Calculate the best business day (day with the most portions sold)
        let mut best_day_index = 0;
        let mut max_portions_sold = 0;
        for (index, daily_stat) in self.stats.iter().enumerate() {
            if daily_stat.portions_sold > max_portions_sold {
                max_portions_sold = daily_stat.portions_sold;
                best_day_index = index;
            }
        }

        // Calculate the best worker (most portions sold on average across all days)
        let num_workers = 6;
        let mut total_worker_sales = vec![0; num_workers];

        // Accumulate portions sold by each worker across all days
        for daily_stat in &self.stats {
            for (worker_id, &portions_sold) in daily_stat.worker_stats.iter().enumerate() {
                total_worker_sales[worker_id] += portions_sold;
            }
        }

        // Calculate the best worker by average portions sold
        let mut best_worker = 0;
        let mut highest_average = 0;
        let num_days = self.stats.len();
        for (worker_id, &total_sold) in total_worker_sales.iter().enumerate() {
            let average_sold = total_sold / num_days;
            if average_sold > highest_average {
                highest_average = average_sold;
                best_worker = worker_id;
            }
        }

        println!("------------------------------------------FINAL STATS-------------------------------------------");
        println!("{} days went by", self.stats.len());

        for (index, daily_stat) in self.stats.iter().enumerate() {
            println!("In day {}:", index + 1);
            println!(
                " General stats: {} portions sold | {} portions left",
                daily_stat.portions_sold, daily_stat.portions_left
            );
            print!(" Worker stats: |");
            for sold in &daily_stat.worker_stats {
                print!(" {} |", sold);
            }
            println!("\n");
        }

        println!(
            "Best business day was: day {} with {} portions sold",
            best_day_index + 1, // Adding 1 to display as a 1-based day number
            max_portions_sold
        );
        println!(
            "Best worker was: worker {} with an average of {} portions sold per day.",
            best_worker, highest_average
        );
    }
}

struct Client {
    id: usize,
    is_priority: bool,
    portions_ordered: usize,
}

impl Client {
    fn new(id: usize) -> Self {
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

struct Worker {
    id: usize,
    prioritize: bool,
    portions_sold: usize,
}

impl Worker {
    fn new(id: usize) -> Self {
        let prioritize = !(id == 0 || id == 1); // workers 0 and 1 prioritize regular customers unlike the rest

        Self {
            id,
            prioritize,
            portions_sold: 0,
        }
    }

    fn fetch_client(
        &self,
        regular_queue: Arc<Mutex<VecDeque<Client>>>,
        priority_queue: Arc<Mutex<VecDeque<Client>>>,
    ) -> Option<Client> {
        // check if there are priority clients waiting
        let mut queue = if self.prioritize {
            // prioritizes priority costumers
            let priority_queue = priority_queue.lock().unwrap();
            if !priority_queue.is_empty() {
                priority_queue
            } else {
                regular_queue.lock().unwrap()
            }
        } else {
            // prioritizes regular costumers -> inverse behaviour
            let regular_queue = regular_queue.lock().unwrap();
            if !regular_queue.is_empty() {
                regular_queue
            } else {
                priority_queue.lock().unwrap()
            }
        };
        queue.pop_front()
    }

    fn service(
        &mut self,
        is_open: Arc<Mutex<bool>>,
        regular_queue: Arc<Mutex<VecDeque<Client>>>,
        priority_queue: Arc<Mutex<VecDeque<Client>>>,
        cakes: Arc<Mutex<VecDeque<usize>>>,
    ) -> (usize, usize) {
        let color = match self.id {
            0 => "\x1b[31m", // Red
            1 => "\x1b[32m", // Green
            2 => "\x1b[33m", // Yellow
            3 => "\x1b[34m", // Blue
            4 => "\x1b[35m", // Magenta
            5 => "\x1b[36m", // Cyan
            _ => "\x1b[0m",  // Default
        };
        let reset = "\x1b[0m";

        if self.prioritize {
            println!(
                "{}Worker {} prioritizes priority customers over regular customers{}",
                color, self.id, reset
            );
        } else {
            println!(
                "{}Worker {} prioritizes regular customers over priority customers{}",
                color, self.id, reset
            );
        }

        loop {
            if *is_open.lock().unwrap() {
                let client = self.fetch_client(regular_queue.clone(), priority_queue.clone());

                if let Some(client) = client {
                    println!(
                        "{}Worker {} is serving client {}{}",
                        color, self.id, client.id, reset
                    );

                    let ordered_portions = client.portions_ordered;
                    let mut full_cakes = ordered_portions / 6;
                    let mut individual_portions = ordered_portions % 6;
                    let service_time = full_cakes + individual_portions;
                    let service_duration = Duration::from_secs((service_time).try_into().unwrap());
                    {
                        /*
                         * lets start by verifying that we can fulfill the client's order
                         * we try to get the first 3 cakes, since the max order are 3 cakes.
                         * if we dont have 3 cakes left, we get the number of portions for the remainder
                         */
                        let mut cakes = cakes.lock().unwrap();
                        let (slice1, slice2) = cakes.as_slices();
                        let total_elements = slice1.len() + slice2.len();
                        let available_portions: usize = if total_elements >= 3 {
                            // Sum the first 3 elements across slice1 and slice2
                            slice1.iter().chain(slice2.iter()).take(3).sum()
                        } else {
                            // Sum all available elements if fewer than 3
                            slice1.iter().chain(slice2.iter()).sum()
                        };

                        if available_portions < ordered_portions {
                            // go to the next customer, as we don't have enough cake for the current one
                            println!("{}Cannot serve client {} | Requested {} portions, only {} are available{}", color, client.id, ordered_portions, available_portions, reset);
                            continue;
                        } else {
                            println!(
                                "{}Worker {} takes {} seconds to serve {} pieces of cake to client {}{}",
                                color, self.id, service_time, ordered_portions, client.id, reset
                            );

                            // lets start by removing as many complete cakes as possible
                            // this is done to save the clients time
                            while full_cakes > 0 {
                                cakes.pop_back();
                                full_cakes -= 1;
                            }

                            if let Some(&prev_front_cake) = cakes.front() {
                                //now that we got the complete cakes, we get the remaining pieces
                                while individual_portions > 0 {
                                    if let Some(cake) = cakes.front_mut() {
                                        if *cake <= individual_portions {
                                            individual_portions -= *cake;
                                            cakes.pop_front();
                                        } else {
                                            *cake -= individual_portions;
                                            break;
                                        }
                                    }
                                }

                                if let Some(current_front_cake) = cakes.front() {
                                    println!(
                                        "{}Previous cake on top of stack was |{}|; After service, current cake on top of stack is |{}|{}",
                                        color, prev_front_cake, current_front_cake, reset
                                    );
                                } else {
                                    println!(
                                        "{}Previous cake on top of stack was |{}|; After service, there is no cake in stack{}",
                                        color, prev_front_cake, reset
                                    );
                                }
                            } else {
                                // Will only reach this case if order was only full cakes
                            }
                        }
                    }
                    self.portions_sold += ordered_portions;
                    thread::sleep(service_duration);
                } else {
                    thread::sleep(Duration::from_millis(1000));
                    continue;
                }
            } else {
                println!(
                    "{}Worker {} is done for the day. Sold {} portions{}",
                    color, self.id, self.portions_sold, reset
                );
                break;
            }
        }
        (self.id, self.portions_sold)
    }
}

fn cake_production(is_morning: Arc<Mutex<bool>>, cakes: Arc<Mutex<VecDeque<usize>>>) {
    {
        let mut cakes = cakes.lock().unwrap();
        *cakes = VecDeque::from(vec![6; 10]);
    }; // have 10 cakes initially

    loop {
        // Check if it's morning
        if *is_morning.lock().unwrap() {
            // Simulate producing one cake with a delay
            thread::sleep(Duration::from_millis(400));
            let mut cakes = cakes.lock().unwrap();
            cakes.push_back(6); // Each cake starts with 6 portions
        } else {
            // Afternoon production: reset with a fixed number of cakes
            let n_cakes = {
                let mut cakes = cakes.lock().unwrap();
                println!(
                    "Done producing cakes for today. Have {} cakes from the morning in stack",
                    cakes.len()
                );
                let mut afternoon_cakes: VecDeque<usize> = VecDeque::from(vec![6; 50]);
                cakes.append(&mut afternoon_cakes);
                cakes.len()
            };
            println!(
                "Starting AFTERNOON period, 50 more cakes available. Currently have {} cakes in stack",
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
                    println!(
                        "Priority customer with id {} arrived, wanting {} cake portions",
                        client.id, client.portions_ordered
                    );
                    priority_queue.lock().unwrap()
                } else {
                    println!(
                        "Customer with id {} arrived, wanting {} cake portions",
                        client.id, client.portions_ordered
                    );
                    regular_queue.lock().unwrap()
                };
                queue.push_back(client);
            }
            id += 1;

            let normal = Normal::new(1.0, 0.1).unwrap();
            let arrival_time = (normal.sample(&mut rand::thread_rng())).to_u64().unwrap();
            thread::sleep(Duration::from_secs(arrival_time));
        } else {
            // println!("Day has ended, no more customers are coming in");
            break;
        }
    }
}

fn main() {
    let mut bakery = Bakery { stats: Vec::new() };

    // from https://medium.com/@rohanbhatotiya/how-can-we-take-integers-as-an-input-in-rust-8f76ddf51010
    let mut input = String::new();
    println!("Enter the number of days to simulate.");
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");
    let num_days: u32 = input.trim().parse().expect("Invalid input");

    for day in 1..=num_days {
        let regular_queue = Arc::new(Mutex::new(VecDeque::<Client>::with_capacity(100)));
        let priority_queue = Arc::new(Mutex::new(VecDeque::<Client>::with_capacity(20)));
        let cakes = Arc::new(Mutex::new(VecDeque::<usize>::with_capacity(100)));

        println!("------------------------------------------START OF DAY {}-------------------------------------------", day);

        // Reset the "open" flag at the beginning of each day
        let is_open = Arc::new(Mutex::new(true));
        let is_morning = Arc::new(Mutex::new(true));

        // Start the timer thread for "10 hours" (10 seconds for testing)
        let is_open_timer = Arc::clone(&is_open);
        let is_morning_timer = Arc::clone(&is_morning);
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(30)); // Simulate 5 hours with a shorter duration
            {
                let mut is_morning = is_morning_timer.lock().unwrap();
                *is_morning = false; // now it's afternoon
                println!("5 hours have passed. It is the afternoon for day {}.", day);
            }
            thread::sleep(Duration::from_secs(30)); // Simulate 5 hours with a shorter duration, for a total of 10 hours
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
            thread::sleep(Duration::from_millis(200));
            customer_arrival(is_open_clone, regular_queue_clone, priority_queue_clone);
        });

        // Start the bakery thread for the day
        bakery.operate(is_open, regular_queue, priority_queue, cakes);

        production_handle.join().unwrap();
        customer_handle.join().unwrap();

        // Small interval to simulate break between days
        println!("------------------------------------------END OF DAY {}-------------------------------------------\n", day);
        thread::sleep(Duration::from_secs(5));
    }
    bakery.general_stats();
}
