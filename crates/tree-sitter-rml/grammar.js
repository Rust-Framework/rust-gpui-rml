module.exports = grammar({
  name: 'rml',

  extras: $ => [/\s+/],

  rules: {
    document: $ => repeat(choice($.element, $.text, $.comment)),

    element: $ => choice(
      $.self_closing_element,
      seq($.start_tag, repeat(choice($.element, $.text, $.interpolation, $.comment)), $.end_tag),
    ),

    start_tag: $ => seq('<', $.tag_name, repeat($.attribute), '>'),
    self_closing_element: $ => seq('<', $.tag_name, repeat($.attribute), '/>'),
    end_tag: $ => seq('</', $.tag_name, '>'),

    tag_name: $ => /[A-Za-z][A-Za-z0-9_-]*/,

    attribute: $ => seq($.attribute_name, optional(seq('=', $.attribute_value))),

    attribute_name: $ => /[A-Za-z_][A-Za-z0-9_:.-]*/,

    attribute_value: $ => choice($.string, $.binding),

    string: $ => choice(seq('"', /[^"]*/, '"'), seq("'", /[^']*/, "'")),

    binding: $ => seq('{', optional($.expression), '}'),

    interpolation: $ => seq('{', optional($.expression), '}'),

    expression: $ => /[^}]+/,

    text: $ => /[^<{]+/,

    comment: $ => token(seq('<!--', repeat(choice(/[^-]/, /-[^>]/)), '-->')),
  },
});
