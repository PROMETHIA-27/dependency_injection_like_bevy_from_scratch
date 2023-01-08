# Introductions

## What is Dependency Injection?

Dependency injection is a needlessly complicated way to phrase "asking for things instead of providing them". 
An easy example of dependency injection would be `Iterator::map()`; you provide a function/closure which asks for
an item to map and maps it, and the iterator itself "injects" that "dependency". In this case we're mimicking what
Bevy Engine does, which is create an `App` which is provided with many `System`'s (functions) which ask for 
`Query`'s and `Res`'s (the dependencies) which the `App` provides and calls the `System`'s repeatedly (the injection).
Dependency injection is a useful pattern, and most people probably know what it is even if they don't know its name.
