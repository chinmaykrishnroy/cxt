from state import commit_level


def dispatch(value):
    return commit_level(value)


def unrelated():
    return "diagnostic noise"
