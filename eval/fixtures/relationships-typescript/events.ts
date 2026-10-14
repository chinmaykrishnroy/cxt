import { commitLevel } from './state';

export function dispatch(value: number) {
    return commitLevel(value);
}

export function unrelated() {
    return 'diagnostic noise';
}
