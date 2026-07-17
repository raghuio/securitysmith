#let scope(findings: (), requirements: (), scope: none, sections: (:), metadata: (:), notes: (), doc) = {
  set page(paper: "a4", margin: 2cm)
  set text(font: "DejaVu Sans", 11pt)

  #text(size: 20pt, weight: "bold")[Scope]
  #v(0.5cm)

  if scope != none [
    #scope
  ] else [
    No scope file found.
  ]

  doc
}