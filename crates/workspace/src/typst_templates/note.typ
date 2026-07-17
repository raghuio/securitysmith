#let note(findings: (), requirements: (), scope: none, sections: (:), metadata: (:), notes: (), doc) = {
  set page(paper: "a4", margin: 2cm)
  set text(font: "DejaVu Sans", 11pt)

  #text(size: 20pt, weight: "bold")[Notes]
  #v(0.5cm)

  #for n in notes [
    === #n.id
    #n.body
    #v(0.3cm)
  ]

  doc
}