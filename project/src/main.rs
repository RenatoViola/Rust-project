use rand::Rng;
use rand_distr::num_traits::ToPrimitive;
use rand_distr::{Distribution, Normal};
use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

struct CustomerQueues {
    regular_queue: VecDeque<Client>,
    priority_queue: VecDeque<Client>,
}

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
        is_open: Arc<RwLock<bool>>,
        customer_queues: Arc<(Mutex<CustomerQueues>, Condvar)>,
        cakes: Arc<Mutex<VecDeque<usize>>>,
    ) {
        let mut handles = vec![];

        // Each of the six workers operates in its own exclusive thread
        for id in 0..6 {
            let is_open_clone = Arc::clone(&is_open);
            let customer_queues_clone = Arc::clone(&customer_queues);
            let cakes_clone = Arc::clone(&cakes);

            let handle = thread::spawn(move || {
                let mut worker = Worker::new(id);
                worker.service(is_open_clone, customer_queues_clone, cakes_clone)
            });

            handles.push(handle);
        }

        // Collect the results from each worker and store them
        self.gather_daily_stats(cakes, handles);
    }

    fn gather_daily_stats(
        &mut self,
        cakes: Arc<Mutex<VecDeque<usize>>>,
        handles: Vec<JoinHandle<(usize, usize)>>,
    ) {
        let mut worker_stats = vec![0; 6];
        let mut best_worker = 10;
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
        let portions_left = cakes.lock().unwrap().iter().sum();

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
                "{} was the best worker and everyone takes {} portions home today",
                best_worker, divided_portions
            );
        } else {
            println!(
                "{} was the best worker, so he's taking {} portions home today. Everyone else takes {} portions of cake",
                best_worker, (divided_portions + portions_remainder), divided_portions
            );
        }
    }

    fn general_stats(&self) {
        let mut best_day_index = 0;
        let mut most_portions_sold = 0;
        for (index, daily_stat) in self.stats.iter().enumerate() {
            if daily_stat.portions_sold > most_portions_sold {
                most_portions_sold = daily_stat.portions_sold;
                best_day_index = index;
            }
        }

        // The best worker is the one who sold more portions on average, across all days
        let mut total_worker_sales = vec![0; 6];
        for daily_stats in &self.stats {
            for (id, portions_sold) in daily_stats.worker_stats.iter().enumerate() {
                total_worker_sales[id] += portions_sold;
            }
        }
        let mut best_worker = 0;
        let mut best_average = 0;
        let num_days = self.stats.len();
        for (id, &total_sold) in total_worker_sales.iter().enumerate() {
            let average_sold = total_sold / num_days;
            if average_sold > best_average {
                best_average = average_sold;
                best_worker = id;
            }
        }

        // Print the most important information for each day
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
            best_day_index + 1,
            most_portions_sold
        );
        println!(
            "Best worker was: worker {} with an average of {} portions sold per day",
            best_worker, best_average
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
        let prioritize = !(id == 0 || id == 1); // Workers 0 and 1 prioritize regular customers unlike the rest

        Self {
            id,
            prioritize,
            portions_sold: 0,
        }
    }

    fn fetch_client(
        &self,
        is_open: Arc<RwLock<bool>>,
        customer_queues: Arc<(Mutex<CustomerQueues>, Condvar)>,
    ) -> Option<Client> {
        let (queues_lock, cvar) = &*customer_queues;
        let mut queues = queues_lock.lock().unwrap();
        // Wait until there is a client to serve in either queue
        while *is_open.read().unwrap()
            && queues.priority_queue.is_empty()
            && queues.regular_queue.is_empty()
        {
            queues = cvar.wait(queues).unwrap();
        }

        if self.prioritize {
            // Prioritize serving a priority customer over a regular one
            if let Some(client) = queues.priority_queue.pop_front() {
                Some(client)
            } else {
                queues.regular_queue.pop_front()
            }
        } else {
            // Prioritize serving a regular customer over a priority one
            if let Some(client) = queues.regular_queue.pop_front() {
                Some(client)
            } else {
                queues.priority_queue.pop_front()
            }
        }
    }

    fn service(
        &mut self,
        is_open: Arc<RwLock<bool>>,
        customer_queues: Arc<(Mutex<CustomerQueues>, Condvar)>,
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

        // The worker announces the type of clients it prioritizes before beginning service
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

        while *is_open.read().unwrap() {
            let client = self.fetch_client(Arc::clone(&is_open), Arc::clone(&customer_queues));

            if let Some(client) = client {
                println!(
                    "{}Worker {} is serving client {}{}",
                    color, self.id, client.id, reset
                );

                let ordered_portions = client.portions_ordered;
                let mut full_cakes = ordered_portions / 6;
                let mut individual_portions = ordered_portions % 6;
                let service_time = full_cakes + individual_portions;
                {
                    /*
                    Before messing with the cake tray, we want to verify that we can actually fulfill the client's order.
                    Since the cake in front is the one that is likely to be incomplete, we try to get the last 3 cakes, since the max order are 3 cakes.
                    If we don't have 3 cakes left, we get the number of portions for the available ones, check if there's enough to serve the client.
                    */
                    let mut cakes = cakes.lock().unwrap();
                    let (slice1, slice2) = cakes.as_slices(); // Some elements wrap around so must divide by slices to get them in order
                    let available_portions: usize = if slice1.len() + slice2.len() >= 3 {
                        // Sum of the portions of the last 3 cakes in the queue (across both slices)
                        slice1.iter().chain(slice2.iter()).rev().take(3).sum()
                    } else {
                        // Less than 3 cakes in queue -> Sum the portions of all cakes left
                        slice1.iter().chain(slice2.iter()).sum()
                    };

                    if available_portions < ordered_portions {
                        // Go fetch another customer in queue, as we don't have enough cake to serve the current one
                        println!("{}Cannot serve client {} | Requested {} portions, only {} are available{}", color, client.id, ordered_portions, available_portions, reset);
                        continue;
                    } else {
                        println!(
                                "{}Worker {} takes {} minutes to serve {} pieces of cake to client {}{}",
                                color, self.id, service_time, ordered_portions, client.id, reset
                            );

                        // Firstly, serve as many whole cakes as possible
                        while full_cakes > 0 {
                            cakes.pop_back();
                            full_cakes -= 1;
                        }

                        if let Some(&prev_front_cake) = cakes.front() {
                            // Secondly, serve the remaining portions
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

                            // Just logging the state of the cake stack/queue before and after the service
                            if let Some(current_front_cake) = cakes.front() {
                                println!(
                                        "{}Previous cake on top of stack was |{}|; After service, current cake on top of stack is |{}| and there are {} cakes in stack{}",
                                        color, prev_front_cake, current_front_cake, cakes.len(), reset
                                    );
                            } else {
                                println!(
                                        "{}Previous cake on top of stack was |{}|; After service, there is no cake in stack{}",
                                        color, prev_front_cake, reset
                                    );
                            }
                        }
                    }
                }
                self.portions_sold += ordered_portions;
                let service_duration = Duration::from_secs((service_time).try_into().unwrap());
                thread::sleep(service_duration);
            }
        }
        println!(
            "{}Worker {} is done for the day. Sold {} portions{}",
            color, self.id, self.portions_sold, reset
        );
        (self.id, self.portions_sold)
    }
}

fn cake_production(is_morning: Arc<RwLock<bool>>, cakes: Arc<Mutex<VecDeque<usize>>>) {
    while *is_morning.read().unwrap() {
        // Producing each cake with takes 400 ms
        thread::sleep(Duration::from_millis(400));
        let mut cakes = cakes.lock().unwrap();
        cakes.push_back(6); // Every cake has 6 portions initially
    }

    // Afternoon behavior: fixed number of cakes (50) is added to the morning leftovers
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
        "Starting afternoon period, 50 more cakes available. Currently have {} cakes in stack",
        n_cakes
    );
}

fn customer_arrival(
    is_open: Arc<RwLock<bool>>,
    customer_queues: Arc<(Mutex<CustomerQueues>, Condvar)>,
) {
    let mut id = 0;

    while *is_open.read().unwrap() {
        let client = Client::new(id);
        let (queues, cvar) = &*customer_queues;
        {
            let mut queues = queues.lock().unwrap();
            if client.is_priority {
                println!(
                    "Priority customer with id {} arrived, wanting {} cake portions",
                    client.id, client.portions_ordered
                );
                queues.priority_queue.push_back(client);
            } else {
                println!(
                    "Customer with id {} arrived, wanting {} cake portions",
                    client.id, client.portions_ordered
                );
                queues.regular_queue.push_back(client);
            }
            cvar.notify_one();
        }
        id += 1;
        // Arrival of customers is a normal distribution, take on average about 1 second
        let normal = Normal::new(1.0, 0.1).unwrap();
        let arrival_time = (normal.sample(&mut rand::thread_rng())).to_u64().unwrap();
        thread::sleep(Duration::from_secs(arrival_time));
    }
    customer_queues.1.notify_all(); // Signal to all waiting threads that the day is over
}

fn day_time_logic(is_open: Arc<RwLock<bool>>, is_morning: Arc<RwLock<bool>>) {
    // Start the timer thread for "10 hours" (60 seconds for simulation)
    thread::sleep(Duration::from_secs(30));
    {
        let mut is_morning = is_morning.write().unwrap();
        *is_morning = false;
        println!("5 hours have passed. It is the afternoon.");
    }
    thread::sleep(Duration::from_secs(30));
    let mut is_open = is_open.write().unwrap();
    *is_open = false;
    println!("10 hours have passed. Closing the bakery for the day.");
}

fn main() {
    let mut bakery = Bakery { stats: Vec::new() };

    // from https://medium.com/@rohanbhatotiya/how-can-we-take-integers-as-an-input-in-rust-8f76ddf51010
    let mut input = String::new();
    println!("Enter the number of days to simulate");
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");
    let num_days: u32 = input.trim().parse().expect("Invalid input");

    for day in 1..=num_days {
        let customer_queues = Arc::new((
            Mutex::new(CustomerQueues {
                regular_queue: VecDeque::with_capacity(100),
                priority_queue: VecDeque::with_capacity(20),
            }),
            Condvar::new(),
        ));

        // Bakery opens with 20 cakes already available
        let mut cake_deque = VecDeque::with_capacity(100);
        for _ in 0..20 {
            cake_deque.push_back(6);
        }
        let cakes = Arc::new(Mutex::new(cake_deque));

        println!("------------------------------------------START OF DAY {}-------------------------------------------", day);

        // Reset the "is_open" and "is_morning" flag at the beginning of each day
        let is_open = Arc::new(RwLock::new(true));
        let is_morning = Arc::new(RwLock::new(true));

        let is_open_timer = Arc::clone(&is_open);
        let is_morning_timer = Arc::clone(&is_morning);
        let time_handle = thread::spawn(move || {
            // Time passes by in the background, other threads check for time
            day_time_logic(is_open_timer, is_morning_timer);
        });

        let cakes_clone = Arc::clone(&cakes);
        let production_handle = thread::spawn(move || {
            // Cake production takes place in the background
            cake_production(is_morning, cakes_clone);
        });

        let is_open_clone = Arc::clone(&is_open);
        let customer_queues_clone = Arc::clone(&customer_queues);
        let customer_handle = thread::spawn(move || {
            // Customer arrival takes place in the background
            customer_arrival(is_open_clone, customer_queues_clone);
        });

        // The bakery starts operating for the day
        bakery.operate(is_open, customer_queues, cakes);

        // Join on all threads (aside from workers, that's the bakery's business)
        time_handle.join().unwrap();
        production_handle.join().unwrap();
        customer_handle.join().unwrap();

        // Small interval to simulate break between days
        println!("------------------------------------------END OF DAY {}-------------------------------------------\n", day);
        thread::sleep(Duration::from_secs(2));
    }
    // At the end of the simulation, calculate and print the stats across the days that passed
    bakery.general_stats();
}
