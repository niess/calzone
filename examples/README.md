# Calzone examples

This folder gathers examples of Calzone usage. These examples are organised by
purpose. For instance, the [`gamma/underwater`](gamma/underwater) sub-folder
illustrates the transport of gamma-rays within a water environment.

Each example contains one or more geometry definition files (e.g.
[`detector.toml`](gamma/underwater/detector.toml) and
[`geometry.toml`](gamma/underwater/geometry.toml)) in [TOML][TOML] format. Thus,
when using Python 3.10 or less, the [tomli][TOMLI] package must first be
installed in order to run the example. This can be done as
```bash
pip install tomli
```

Then, the corresponding Python example script (e.g.
[`run.py`](gamma/underwater/run.py)) can simply be executed as
```bash
python run.py
```


[TOML]: https://toml.io/
[TOMLI]: https://pypi.org/project/tomli/
