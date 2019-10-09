# Space efficient quantile

This repo will implement one (or more) space-efficient algorithm to compute quantiles (like median).

This is mostly an exercise of Rust :)

*IN PROGRESS*

```
# Naive with 100M elements
OrderedF64(17.000000016215086)

real    0m14.656s
user    0m17.475s
sys     0m0.568s

# Space-efficient quantile with 100M elements and 1% max error (16 threads)
(OrderedF64(16.989460957214533), 0.00887439)

real    0m1.006s
user    0m13.829s
sys     0m0.004s

# Space-efficient quantile with 100M elements and 0.01% max error (16 threads)
(OrderedF64(17.00000142410929), 0.00005147)

real    0m1.768s
user    0m25.119s
sys     0m0.064s

# Space-efficient quantile with 1B elements and 0.01% max error (16 threads)
(OrderedF64(17.00000142410929), 0.000051472)

real    0m16.306s
user    4m11.150s
sys     0m0.052s

# Space-efficient quantile with 1B elements and 0.01% max error (1 thread)
(OrderedF64(17.00000142410929), 0.000000747)

real    2m42.841s
user    2m42.720s
sys     0m0.084s
```