Ensures that the code is fully written in ASCII characters.

V8, the JavaScript engine Deno relies on, provides a method that strings get
populated outside V8's heap. In particular, if they are composed of one-byte
characters only, V8 can handle them much more efficiently through
[`v8::String::ExternalOneByteStringResource`]. In order to leverage this V8
feature in the internal of Deno, this rule checks if all characters in the code
are ASCII.

[`v8::String::ExternalOneByteStringResource`]: https://v8.github.io/api/head/classv8_1_1String_1_1ExternalOneByteStringResource.html

That said, you can also make use of this lint rule for something other than
Deno's internal JavaScript code. For instance, `-` (an ASCII character, Unicode
code point is U+002D) and `‐` (_not_ an ASCII character, Unicode code point is
U+2010) look almost the same to us, but are completely different characters.
Only the ASCII version is valid as a binary operator in JavaScript. This rule
will help you avoid such confusion.

### Invalid:

```typescript
const a = 42 ‐ 2; // U+2010

// “comments” are also checked
// ^        ^
// |        U+201D
// U+201C
```

### Valid:

```typescript
const a = 42 - 2;

// "comments" are also checked
```
