#let finding(findings: (), requirements: (), scope: none, sections: (:), metadata: (:), notes: (), doc) = {
  set page(paper: "a4", margin: 2cm)
  set text(font: "DejaVu Sans", 11pt)

  if findings.len() > 0 [
    let f = findings.first()
    #text(size: 20pt, weight: "bold")[#f.id]
    #v(0.3cm)
    *Severity:* #f.severity *Status:* #f.status
    #v(0.5cm)
    #f.body
  ]

  doc
}