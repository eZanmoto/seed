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

Operations
----------

### Integers

```
print(1 + 2); # 3
print(5 - 2); # 3
print(2 - 5); # -3
print(2 * 3); # 6
print(5 / 2); # 2
print(5 % 2); # 1
```

Note that division only performs integer division.

### Strings

```
print("Hello," + " world!"); # Hello, world!
```

### Boolean

```
print(true && false); # false
print(true || false); # true
```

### Comparison

```
print(1 < 2); # true
print(3 <= 2); # false
print(1 > 2); # false
print(3 >= 2); # true
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
    print(n); # 1

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

Functions create a closure over the scope in which they're defined:

```
v := 1;
fn inc_v() {
    v = v + 1;
}

print(v); # 1
inc_v();
print(v); # 2
```
