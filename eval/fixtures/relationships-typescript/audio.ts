import { commitLevel } from './state';

export function changeVolume(value: number) {
    return commitLevel(value);
}
