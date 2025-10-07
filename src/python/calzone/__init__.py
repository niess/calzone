def init():
    from . import _core
    import sys

    this = sys.modules[__name__]
    setattr(this, "__all__", _core.__all__)
    for name in _core.__all__:
        setattr(this, name, getattr(_core, name))
    setattr(this, "__doc__", getattr(_core, "__doc__"))

init()
del init

VERSION = "1.1.4"
