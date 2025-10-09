# Compile Time Impact Demo

At time of writing, it has been more than 2 years since I first wrote this book. In that time I've
done more work with bevy and become increasingly frustrated with how long compile times are on my
laptop; it's slightly old, from 2021 or so, but an AMD Ryzen 9 5900HX should still easily crunch
through an 8k LOC bevy project in <5s. Instead, I've been dealing with more like 40-50s. For iterative,
non-optimized compiles.

I'm not sure where all this slowdown is coming from; certainly some of it is linking times, from
the 600 or so dependencies I have. But it's not *all* linking times, because even just `cargo check`
takes forever, even on a shared library in the game rather than the binary crate.

I began to suspect that the dependency injection trick bevy uses is causing pathologically bad behavior
in the compiler. To test this, I created a sample of my own dependency injection system (which is
overall far simpler than bevy's) and looked at how it scales up. This demo lives in the 
repository for this mdbook, under the name `compile_time_test`. You can check compile times on
your system with `just` (compiles with dependency injection) or `just call` (compiles where 
systems are simply called in order as normal functions, rather than injecting). Note that you may
want to run the same command a few times consecutively to see a "cached" compilation; depending on context, 
a single run can take substantially longer for some reason, I assume from the compiler redoing work
that it can cache for similar compilations. You'll also need a nightly compiler as I use the unstable
`random` feature in a script.

Here are the results I get on my machine (passes that take <1s are skipped, since they hardly matter
for a slow build)

### Dependency Injection
```
time:   7.660; rss:  205MB ->  890MB ( +685MB)  type_check_crate
time:   3.487; rss:  890MB -> 1051MB ( +161MB)  MIR_borrow_checking
time:   3.230; rss: 1057MB -> 1210MB ( +153MB)  monomorphization_collector_graph_walk
time:   2.152; rss: 1241MB -> 1728MB ( +488MB)  codegen_to_LLVM_IR
time:   5.636; rss: 1057MB -> 1728MB ( +672MB)  codegen_crate
time:   7.898; rss: 1456MB ->  279MB (-1178MB)  LLVM_passes
time:   6.789; rss:  844MB ->  276MB ( -569MB)  finish_ongoing_codegen
time:  25.059; rss:   12MB ->   58MB (  +46MB)  total
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 25.30s
```

### Just Call
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.82s
```

Pretty concerning, in my opinion. The demo includes 5000 empty systems each taking 3 parameters
and either calls them by using
`.add_system()` to add them to the scheduler which will call them dynamically, or it generates
a function that will call every system in sequence.

It seems that generally, the type system, monomorphization, and code generation all blow up when
exposed to this design pattern, and cause a substantial slowdown.

From my own testing, the overhead of the dependency injection pattern is linear in the number of systems,
with 1000 systems taking ~5s on my machine instead of ~25s. From a cursory search, I would estimate
bevy has about 650-800 systems in the actual dependency tree, so that overhead really does matter.
It is also quite noticeable in a project I work on with only 94 instances of `add_systems` (likely
closer to 150-200 total systems?).

This is hardly the only source of slowdown in bevy's compile times (linking is still *awful*)
but I believe it is significant.
Unfortunately, I don't see a solution here short of rewriting almost all bevy code, which is 
probably non-negotiable.

Keep this in mind when designing your own libraries and apps; if you value quick compile times, 
keep in mind whether the features you use are going to cause a lot of stress for the compiler to
compute. Things like generics, traits, macros, and type system tricks cause a lot of strain. 
Consider if the simplest option might just be better for your use case, or if you can leave a few
nanoseconds of performance on the table to get back minutes of compile times.

Also, feel free to play with this demo yourself and suggest improvements/interesting discoveries!
I am interested in gaining a better understanding of the forces at play and documenting that information
here. Perhaps there is a way to rearchitect dependency injection to stress the compiler dramatically less,
or this could serve as a good stress test that can lead to compiler improvements to handle this 
more gracefully; who knows. I would love to see improvements that let me compile bevy faster, one
way or another.