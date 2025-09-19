# Calzone documentation

Building Calzone's documentation requires a working installation of Calzone (see
[installation][RTD_INSTALLATION] instructions) as well as the [`sphinx`][SPHINX]
package with the [Book][BOOK_THEME] theme (see the
[source/requirements.txt](source/requirements.txt) file).

On Unix systems, the HTML documentation should build with the provided
[Makefile](Makefile) as
```bash
make html
```

Testing the documentation examples (further requiring the [`pytest`][PYTEST]
package) can be done as
```bash
pytest --doctest-glob="*.rst"
```


[BOOK_THEME]: https://sphinx-book-theme.readthedocs.io/
[PYTEST]: https://docs.pytest.org
[RTD_INSTALLATION]: https://calzone.readthedocs.io/en/latest/installation.html
[SPHINX]: https://www.sphinx-doc.org
