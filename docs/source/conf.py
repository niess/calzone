project = "Calzone"
copyright = "Université Clermont Auvergne, CNRS/IN2P3, LPCA"
author = "Valentin Niess"
release = "0.5.0"

highlight_language = "python3"

extensions = [
    "sphinx.ext.autodoc",
    "sphinx.ext.autosectionlabel",
    "sphinx.ext.doctest",
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
.. |nbsp| unicode:: 0xA0
   :trim:

.. role:: bash(code)
    :language: bash
    :class: highlight

.. role:: python(code)
    :language: python
    :class: highlight

.. role:: underline
    :class: underline
"""

html_theme = "sphinx_book_theme"
html_theme_options = {
    "logo": {
        "text": f"Calzone {release} documentation",
        "image_light": "_static/images/logo.svg",
        "image_dark": "_static/images/logo.svg",
    }
}
html_static_path = ["_static"]
html_css_files = [
    'css/custom.css',
]
html_favicon = "_static/images/logo.svg"
