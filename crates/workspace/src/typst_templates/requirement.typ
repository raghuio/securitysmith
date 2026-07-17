#let requirement(findings: (), requirements: (), scope: none, sections: (:), metadata: (:), notes: (), doc) = {
  set page(paper: "a4", margin: 2cm)
  set text(font: "DejaVu Sans", 11pt)

  if requirements.len() > 0 [
    let r = requirements.first()
    #text(size: 20pt, weight: "bold")[#r.id]
    #v(0.3cm)
    *Status:* #r.status
    #v(0.5cm)
    #r.body
  ]

  doc
}