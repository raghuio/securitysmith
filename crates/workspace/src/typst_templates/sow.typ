#let sow(findings: (), requirements: (), scope: none, sections: (:), metadata: (:), notes: (), doc) = {
  set page(paper: "a4", margin: 2cm)
  set text(font: "DejaVu Sans", 11pt)
  set heading(numbering: "1.")

  align(center)[
    #text(size: 24pt, weight: "bold")[Statement of Work]
    #v(0.5cm)
    #text(size: 14pt)[
      #metadata.at("client_name", default: "") — #metadata.at("project_name", default: "")
    ]
    #v(0.3cm)
    #text(size: 11pt)[
      #metadata.at("engagement_name", default: "")
      #if metadata.at("start_date", default: "") != "" [ (#metadata.at("start_date", default: "") — #metadata.at("end_date", default: ""))]
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

  if requirements.len() > 0 [
    = Requirements (#requirements.len())

    #for r in requirements [
      === #r.id — #r.status
      #r.body
      #v(0.3cm)
    ]
  ]

  doc
}