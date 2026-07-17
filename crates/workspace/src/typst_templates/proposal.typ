#let proposal(findings: (), requirements: (), scope: none, sections: (:), metadata: (:), notes: (), doc) = {
  set page(paper: "a4", margin: 2cm)
  set text(font: "DejaVu Sans", 11pt)
  set heading(numbering: "1.")

  align(center)[
    #text(size: 24pt, weight: "bold")[Proposal]
    #v(0.5cm)
    #text(size: 14pt)[
      #metadata.at("client_name", default: "")
    ]
  ]
  pagebreak()

  if scope != none [
    = Scope
    #scope
    pagebreak()
  ]

  for (name, content) in sections {
    if content != none [
      = #name.replace("_", " ")
      #content
      pagebreak()
    ]
  }

  doc
}