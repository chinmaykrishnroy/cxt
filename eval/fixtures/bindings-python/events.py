import state as storage

def dispatch(value):
    return storage.commit(value)
