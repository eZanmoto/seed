Features
========

Comments
------

```
# Comments start with a number sign.
```

Values
------

```
# Null:
null

# Booleans:
true
false

# Integers:
1234

# Strings:
"Hello, world!"

# Lists:
["test", null, true, 1234, [null, true]]

# Objects:
{"hello": "world", "test": [1, 2, 3]}
```

Variables
---------

Variables must be declared with `:=` before they can be used. The location of
the declaration defines the scope of the variable.

```
# Using `n` before this point will result in a "not defined" error.
n := 1
print(n) # 1

n = 2
print(n) # 2
```

Variables are lexically scoped - an assignment will update the variable declared
in the closest surrounding scope, and declaring a variable with the same name as
a variable defined in a surrounding scope will shadow the outer variable.

```
n := 1;
{
    n = 2;
    print(n); # 2

    n := 3;
    print(n); # 3

    n = 4;
    print(n); # 4
}
print(n); # 2
```

Functions
---------

```
fn p(s) {
    print(s);
}

p("Hello, world!");
```
