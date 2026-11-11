# Cake Bakery Simulation in Rust

A multithreaded simulation of a bakery over several days. Workers serve regular and priority customers while arrivals, cake production, and the passage of time run concurrently. The simulation reports sales, leftovers, and worker performance.

The implementation uses mutexes and read-write locks to protect shared state. A condition variable lets workers wait for customers when the queues are empty and wake when the bakery closes.

Developed by Daniel Eugénio and Renato Viola.

[Rust source](project/src/main.rs) · [Project report](Report.pdf)
