def pytest_collection_modifyitems(session, config, items):
    """Monkey patch tests' order."""

    def rank(a):
        return \
            2**3 * int("example" in a.keywords) + \
            2**2 * int("requires_display" in a.keywords) + \
            2**1 * int("requires_goupil" in a.keywords) + \
            2**0 * int("requires_data" in a.keywords)

    items.sort(key=rank)
