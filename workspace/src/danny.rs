
#[derive(Clone)]
struct Day{
    day_number: usize,
    day_sales: usize
}

impl Day{
    fn new(new_day_number: usize) -> Self{
        Day{
            day_number: new_day_number,
            day_sales: 0
        }
    }

    fn updateDay(&mut self){
        self.day_number+=1;
        self.day_sales=0;
    }
}

#[derive(Clone)]
struct Bakery { // basically stats holder
    phase: bool, //true -> morning = infinite cakes | false -> afternoon = 50 cakes
    portions_left: usize,
    time_left: usize,
    current_day: Day, // or 2 position vector - [0] = current day, [1] = portionsSold
    best_day: Day 
}

impl Bakery{
    fn new() -> Self {
        Bakery { 
            phase: true,
            portions_left: std::usize::MAX,
            time_left: 600,
            current_day: Day::new(0),
            best_day: Day::new(0)
        }
    }

    // das duas uma ou fazemos um novo dia, ou atualizamos o valor do dia e damos reset aos bolos
    fn changeDay(&mut self){
        self.current_day.updateDay();
    }

    fn updateBestDay(&mut self){
        if self.best_day.day_sales < self.current_day.day_sales {
            self.best_day = self.current_day.clone(); // nao sei pqq temos de fazer clone mas assim funciona 
        }
    }

    fn decTime(mut self, minutes_passed: usize){
        self.time_left-=minutes_passed;
    }

}

#[derive(Clone)]
struct Worker {
    id: usize,
    priority: bool,
    portions_sold: usize,
}

impl Worker{

    fn new(id: usize, priority_worker: bool) -> Self{
        Worker{
            id,
            priority: priority_worker,
            portions_sold: 0
        }
    }

    fn sellPortions(&mut self, portions_sold: usize){
        self.portions_sold+=portions_sold
    }

    fn newDayStats(&mut self){
        self.portions_sold=0;
    }
}

#[derive(Clone, Copy)]
struct Client {
    id: usize,
    priority: bool,
}

impl Client{

    fn new(id: usize, priority: bool) -> Self{
        Client{ id, priority }
    }
}



fn main() {
    println!("Hello, world!");
}
