import { commit as update } from './state';

export function dispatch(value: number) {
    return update(value);
}
