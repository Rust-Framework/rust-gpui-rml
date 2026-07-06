; RML highlight query — maps syntactic constructs to standard capture names.
;
; Capture vocabulary aligns with gpui-component's HighlightTheme:
;   tag, type, attribute, keyword, function, string, variable, comment,
;   punctuation.bracket

(tag_name) @tag

; PascalCase tag names are components → @type
((tag_name) @type
  (#match? @type "^[A-Z]"))

(attribute_name) @attribute

; Directive names: if/else/each/model/show/once/html/ref/key/slot
((attribute_name) @keyword
  (#match? @keyword "^(if|else|each|model|show|once|html|ref|key|slot)$"))

; Event handlers: onclick, on_change, on-activate, etc.
((attribute_name) @function
  (#match? @function "^on[_:]"))

(string) @string

(binding (expression) @variable)
(interpolation (expression) @variable)

(comment) @comment

; Punctuation
[
  "<"
  ">"
  "</"
  "/>"
  "{"
  "}"
] @punctuation.bracket
