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

Note that division only performs integer division. Operations follow standard
precedence rules:

```
print(2 + 3 * 4); # 14
print((2 + 3) * 4); # 20
print(2 * 3 + 4); # 10
print(2 * (3 + 4)); # 14
```

Assignment-operations can be used as a shorthand for assigning the result of an
operation to a variable:

```
x := 1;

x += 2; # x = x + 2
print(x); # 3

x *= 3; # x = x * 3
print(x); # 9

x %= 2; # x = x % 2
print(x); # 1
```

### Strings

```
print("Hello," + " world!"); # Hello, world!
```

An assignment-operation can be used as a shorthand for assigning the result of
concatenation to a variable:

```
x := "Hello";
x += ", world!";
print(x); # Hello, world!
```

### Boolean

```
print(true && false); # false
print(true || false); # true
```

### Equality

```
print(1 == 1); # true
print(1 == 2); # false
print(1 != 1); # false
print(1 != 2); # true
```

Equality on compound types (lists and objects) perform structural equality:

```
print([] == []); # true
print([] == [1, 2]); # false
print([1, 2] == [1, 2]); # true

print({} == {}); # true
print({} == {"a": 1}); # false
print({"a": 1} == {"a": 1}); # true
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

Control Flow
------------

### If statements

`if` statements don't require parentheses:

```
if true {
    print("expected");
} else if false {
    print("unexpected");
} else {
    print("unexpected");
}
```

`if` statements aren't expressions, and so can't be assigned to values.

### While loops

```
i := 0;
while i < 3 {
    print(i);
    i += 1;
}
print(i); # 3
```

`break`s can be used to exit a loop early:

```
i := 0;
while true {
    if i >= 3 {
        break;
    }

    print(i);
    i += 1;
}
print(i); # 3
```

`continue`s can be used to skip loop iterations:

```
i := 0;
while i < 6 {
    i += 1;

    if i == 3 || i == 4 {
        continue;
    }

    print(i);
}
```

### For loops

`for` can be used to iterate over an iterable value. Iterable values are those
of type `list`, `string` or `object`.

```
for ic in "abc" {
    print(ic);
}
```

`break`s can be used to exit a loop early:

```
for iv in [1, 2, 3, 4] {
    if iv == [2, 3] {
        break;
    }
    print(iv);
}
```

`continue`s can be used to skip loop iterations:

```
for iv in [1, 2, 3, 4] {
    if iv == [1, 2] || iv == [2, 3] {
        continue;
    }
    print(iv);
}
```

Functions
---------

```
fn add(a, b) {
    return a + b;
}

print(add(1, 2)); # 3
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
