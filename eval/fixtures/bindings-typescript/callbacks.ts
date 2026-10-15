import { commit as update } from './state';

export function route(callback: any, value: number) {
    { const update = callback; return update(value); }
}
