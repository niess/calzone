project = "CalZone"
copyright = "Université Clermont Auvergne, CNRS/IN2P3, LPCA"
author = "Valentin Niess"
release = "0.1.0"

highlight_language = "python3"

extensions = [
    "sphinx.ext.autodoc",
    "sphinx.ext.autosectionlabel",
    "sphinx.ext.intersphinx",
]

numfig = True

autodoc_member_order = "groupwise"
autosectionlabel_prefix_document = True
intersphinx_mapping = {
    "python": ("https://docs.python.org/3", None),
    "numpy": ("https://numpy.org/doc/stable/", None)
}

templates_path = ["_templates"]
exclude_patterns = []

rst_prolog = """
.. role:: python(code)
    :language: python
    :class: highlight
"""

html_theme = "sphinx_book_theme"
html_static_path = ["_static"]
