import { commit as persist } from './state';

export function changeVolume(value: number) {
    return persist(value);
}
