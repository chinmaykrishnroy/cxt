def commit(value):
    raise RuntimeError("unrelated subsystem")

def invoke(other, value):
    return other.commit(value)
