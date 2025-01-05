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

# Functions:
fn f() {}
```

Integers can be formatted using underscore to help reading:

```
1_234_567
```

```
# Escape sequences in strings:
"\"\x41n example string\"\n"
```

Operations
----------

### Integers

```
print(1 + 2) # 3
print(5 - 2) # 3
print(2 - 5) # -3
print(2 * 3) # 6
print(5 / 2) # 2
print(5 % 2) # 1
```

Note that division only performs integer division. Operations follow standard
precedence rules:

```
print(2 + 3 * 4) # 14
print((2 + 3) * 4) # 20
print(2 * 3 + 4) # 10
print(2 * (3 + 4)) # 14
```

Operation-assignments can be used as a shorthand for assigning the result of an
operation to a variable:

```
x := 1

x += 2 # x = x + 2
print(x) # 3

x *= 3 # x = x * 3
print(x) # 9

x %= 2 # x = x % 2
print(x) # 1

person := {"age": 20}
person.age += 1
print(person.age) # 21
```

### Strings

```
print("Hello," + " world!") # Hello, world!
```

An operation-assignment can be used as a shorthand for assigning the result of
concatenation to a variable:

```
x := "Hello"
x += ", world!"
print(x) # Hello, world!
```

Strings can be interpolated by prepending a `$` to a string. Each slot of the
string (of the form `${...}`) will be evaluated and substituted in place. At
present, slots may only evaluate to strings:

```
fname := "Jo"
lname := "Soap"
print($"Hello ${fname} ${lname}") # Hello Jo Soap
```

### Lists

```
print([1, 2] + [3, 4]) # [1, 2, 3, 4]
```

An operation-assignment can be used as a shorthand for assigning the result of
concatenation to a variable:

```
x := [1, 2]
x += [3, 4]
print(x) # [1, 2, 3, 4]
```

### Boolean

```
print(true && false) # false
print(true || false) # true
```

### Equality

```
print(1 == 1) # true
print(1 == 2) # false
print(1 != 1) # false
print(1 != 2) # true
```

Equality on compound types (lists and objects) perform structural equality:

```
print([] == []) # true
print([] == [1, 2]) # false
print([1, 2] == [1, 2]) # true

print({} == {}) # true
print({} == {"a": 1}) # false
print({"a": 1} == {"a": 1}) # true
```

Reference equality can be performed on reference types (lists, objects and
functions):

```
a := {}
b := a
c := {}

print(a === a) # true
print(a === b) # true
print(a === c) # false
```

### Comparison

```
print(1 < 2) # true
print(3 <= 2) # false
print(1 > 2) # false
print(3 >= 2) # true
```

### Range

The range operator can be used to generate a list of integers from an inclusive
start to an exclusive end:

```
print(0 .. 4) # [0, 1, 2, 3]
print(-1 .. 2) # [-1, 0, 1]
```

If the start is greater than or equal to the end then the resulting list will be
empty:

```
print(4 .. 0) # []
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
n := 1
{
    print(n) # 1

    n = 2
    print(n) # 2

    n := 3
    print(n) # 3

    n = 4
    print(n) # 4
}
print(n) # 2
```

Variables must start with an alphabetic character or underscore, followed by any
number of alphanumeric characters or underscores.

### Object property names

In an object literal, the evaluated name of a property must be a string. That
means that the property name must be given as a string literal - a standalone
variable name will be evaluated:

```
name := "a"
names := ["b"]
print({name: "Hello", names[0]: "World"}) # { "a": "Hello", "b": "World" }
```

### Object shorthand

In an object literal, providing a variable name without a value will add the
value of that variable (from the current scope) to a new property in the object
with the given name:

```
a := 1
c := 3
print({a, "b": 2, c}) # { a: 1, b: 2, c: 3}
```

### Spread

The items of a list can be inlined into a new list using the spread annotation:

```
xs := [1, 2]
print([xs.., 3, xs..]) # [ 1, 2, 3, 1, 2 ]
```

Arguments to a function can also use the spread annotation:

```
fn f(a, b) {
    print(a)
    print(b)
}

xs := [1, 2]

f(xs..)
```

Objects can also be inlined in this way, but key order matters - later key
values will shadow values declared earlier in the object literal:

```
xs := {"a": 1, "b": 2, "c": 3}
print({xs.., "c": 4}) # {"a": 1, "b": 2, "c": 4}
```

### Indexing

`list`s, `string`s and `object`s can be indexed:

```
xs := null

xs = ["a", "b", "c"]
print(xs[1]) # b

xs = "abc"
print(xs[1]) # b

xs = {"a": 1, "b": 2, "c": 3}
print(xs["b"]) # 2
```

Note that indexing with strings uses byte boundaries, not UTF-8 character
boundaries, so care should be taken when handling strings using UTF-8 encoding.

### Index assigment

`list`s and `object`s can assign to indices:

```
xs := null

xs = ["a", "b", "c"]
xs[1] = "d"
print(xs) # ["a", "d", "c"]

xs = {"a": 1, "b": 2, "c": 3}
xs["b"] = 4
print(xs) # {"a": 1, "b": 4, "c": 3}
```

#### List destructuring

A shorthand can be used for declaring and assigning to variables the values of
lists:

```
xs := [1, 2, 3]
[a, b, c] := xs
print(a) # 1
print(b) # 2
print(c) # 3
```

Underscores can be used to skip items in a list destructure:

```
xs := [1, 2, 3]
[_, _, c] := xs
print(c) # 3
```

The "collect" annotation can be used on the last item in a list destructure to
capture all remaining items from the right-hand side:

```
xs := [1, 2]
[a, _, ..rest] := xs
print(rest) # []
xs = [1, 2, 3, 4]
[a, _, ..rest] = xs
print(rest) # [3, 4]
```

### Object properties

A shorthand can be used for indexing objects when the property is a valid
variable name:

```
xs = {"a": 1, "b": 2, "c": 3}
print(xs.b) # 2
```

The same shorthand can be used for updating objects:

```
xs := {"a": 1, "b": null, "c": 3}
xs.b = 2
xs.d = 4
print(xs) # {"a": 1, "b": 2, "c": 3, "d": 4}
```

#### Object destructuring

A shorthand can be used for declaring and assigning to variables the values of
object properties:

```
xs := {"a": 1, "b": 2, "c": 3}
{a, c} := xs
print(a) # 1
print(c) # 3
```

Keys can be assigned to different variables by providing the key on the left and
the variable name on the right:

```
xs := {"a": 1, "b": 2}
{"a": b, "b": a} := xs
print(a) # 1
print(b) # 2
```

The "collect" annotation can be used on the last item in an object destructure
to capture all remaining items from the right-hand side:

```
xs := {"a": 1, "b": 2, "c": 3, "d": 4}
{b, d, ..rest} := xs
print(b) # 2
print(d) # 4
print(rest) # {"a": 1, "c": 3}
```

Underscores can be used to skip items in an object destructure when using
"collect":

```
xs := {"a": 1, "b": 2, "c": 3, "d": 4}
{"b": _, "d": _, ..rest} := xs
print(rest) # {"a": 1, "c": 3}
```

### Range-indexing

`list`s and `string`s can be range-indexed:

```
print([1, 2, 3, 4, 5, 6][2:4]) # [3, 4]
print("abcdef"[2:4]) # cd
```

The start and end of a range index operation can be omitted, in which case `0`
and the length of the value will be used, respectively:

```
print("abcdef"[:4]) # abcd
print("abcdef"[2:]) # cdef
print("abcdef"[:]) # abcdef
```

Range-indexing can also be used with assignments:

```
xs := [1, 2, 3, 4, 5]
xs[1:4] = [7, 8, 9]
print(xs) # [1, 7, 8, 9, 5]
xs[1:4] = "abc"
print(xs) # [1, "a", "b", "c", 5]
```

Note that range-indexing with a string on the right hand side will index the
string along byte boundaries, not UTF-8 character boundaries, so care should be
taken when handling strings using UTF-8 encoding.

Control Flow
------------

### If statements

`if` statements don't require parentheses:

```
if true {
    print("expected")
} else if false {
    print("unexpected")
} else {
    print("unexpected")
}
```

`if` statements aren't expressions, and so can't be assigned to values.

### While loops

```
i := 0
while i < 3 {
    print(i)
    i += 1
}
print(i) # 3
```

`break`s can be used to exit a loop early:

```
i := 0
while true {
    if i >= 3 {
        break
    }

    print(i)
    i += 1
}
print(i) # 3
```

`continue`s can be used to skip loop iterations:

```
i := 0
while i < 6 {
    i += 1

    if i == 3 || i == 4 {
        continue
    }

    print(i)
}
```

### For loops

`for` can be used to iterate over an iterable value. Iterable values are those
of type `list`, `string` or `object`.

```
for ic in "abc" {
    print(ic)
}
```

Note that iterating over strings uses byte boundaries, not UTF-8 character
boundaries, so care should be taken when handling strings using UTF-8 encoding.

`break`s can be used to exit a loop early:

```
for iv in [1, 2, 3, 4] {
    if iv == [2, 3] {
        break
    }
    print(iv)
}
```

`continue`s can be used to skip loop iterations:

```
for iv in [1, 2, 3, 4] {
    if iv == [1, 2] || iv == [2, 3] {
        continue
    }
    print(iv)
}
```

Functions
---------

```
fn add(a, b) {
    return a + b
}

print(add(1, 2)) # 3
```

Functions create a closure over the scope in which they're defined:

```
v := 1
fn inc_v() {
    v = v + 1
}

print(v) # 1
inc_v()
print(v) # 2
```

Functions are values, and so can be stored in variables, passed as parameters,
etc. A function can be stored in a variable by name:

```
fn f() {
    return 1
}
g := f
print(g()) # 1
```

An function can also be created without a name, i.e. an anonymous function:

```
f := fn () {
    return 1
}
print(f()) # 1
```

Anonymous functions can be useful for creating callbacks:

```
fn f(callback) {
    callback()
}

f(fn () {
    print(1)
})
```

### Parameter destructuring

Function parameters can be destructured in the same way as lists, using the
"collect" annotation (i.e. "varargs"):

```
fn f(a, b, ..c) {
    print(c)
}
f(1, 2) # []
f(1, 2, 3, 4) # [3, 4]
```

This can be combined with the "spread" annotation:

```
fn f(a, b, ..c) {
    print(c)
}
xs := [2, 3]
f(1, xs.., 4) # [3, 4]
```

### `this`

If an object property is a function, then when that function is called, `this`
will refer to the object within the scope of the function:

```
person := {
    "_age": 20,

    "age": fn () {
        return this._age
    },
}

print(person.age()) # 20
```

The value of `this` is based on the "call path" of the function call. For
example, if the same function is assigned to object `a` and object `b`, then
`this` in `a.f()` will refer to `a`, and `this` in `b.f()` will refer to `b`:

```
f := fn () {
    return this._value
}
a := {"_value": 1, f}
b := {"_value": 2, f}

print(a.f()) # 1
print(b.f()) # 2
```

Note that this also holds when the function was defined on one of the objects:

```
a := {
    "_value": 1,
    "f": fn () {
        return this._value
    },
}
b := {"_value": 2, "f": a.f}

print(a.f()) # 1
print(b.f()) # 2
```

As such, it should be noted that the value of `this` depends on the evaluation
context of the function, not on its definition context.

Finally, this evaluation context is carried by variables. In the following
example, even though `f()` is not being called as a property of an object, the
fact that it was dereferenced from `a` earlier means that `this` will refer to
`a` when it is called:

```
a := {
    "_value": "Hello, world!",
    "value": fn () {
        return this._value
    },
}
f := a.value

print(f()) # Hello, world!
```

### Type functions

In this language, objects provide the primary mechanism for user-defined data
and behaviour abstraction. In order to provide access to language-defined
type-based behaviour without conflicting with the user-defined namespace, the
arrow operator is used to access type functions:

```
print("abc"->len()) # 3
```

In this context, the type of the value before the arrow operator defines the
function namespace that is being referenced.

#### `bool`

* `v->type()`: returns `"bool"`

#### `int`

* `v->type()`: returns `"int"`

#### `str`

* `v->len()`: returns the length of `v`.
* `v->type()`: returns `"string"`

#### `list`

* `v->type()`: returns `"list"`

#### `object`

* `v->type()`: returns `"object"`

#### `func`

* `v->type()`: returns `"func"`
