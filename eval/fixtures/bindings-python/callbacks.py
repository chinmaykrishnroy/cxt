from state import commit as update

def dispatch(callback, value):
    update = callback
    return update(value)
