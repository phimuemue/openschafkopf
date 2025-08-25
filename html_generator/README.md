# Simple, opinionated HTML templating framework

## tldr;

To get `<h1>Heading</h1>`, I write `h1("Heading")`.

More complex things similarly. Say I want the following:

```
<div class="DivClass" id="DivId">
<p>This is the first paragraph.</p>
<p>Second paragraph contains a <a href="www.example.com">link</a> and<br/>a linebreak.</p>
</div>
```

To generate it, I write this: 

```
div((
    class("DivClass"),
    id("DivId"),
    p("This is the first paragraph."),
    p((
        "Second paragraph contains a ",
        a((href("www.example.com"), "link")),
        " and",
        br(()),
        "a linebreak."
    )),
))
```

## Main goals

* No manual closing of tags.
* No macros, but functions named after their HTML counterpart.
* No long chains of `html_element.child("p").with_attribute("class").with_child("p").with_content(...)` or similar.

## Implementation notes

Ideally I'd want to write `div("Text")` or `div(class("DivClass"), "Text")`, but Rust does not support function overloading.
Thus, I decided that `div` (and others) always take one argument, and this argument can be simple enough (such as `"Text"` or a tuple, such as `(class("DivClass"), "Text")`.
This means, that oftentimes, function calls look like this `div(("Look", "Two parentheses"))`. I accept that.

When an argument to `div` is a tuple, the crate tries to figure out what the tuple components mean and distribute them into either attributes or children.

After this is done, one gets an object that holds both attributes and children, separately.
This object knows how to `Display` itself.
