/**
 * Compares two values deeply, with an optional epsilon for float comparison.
 * @param val1 - The first value to compare.
 * @param val2 - The second value to compare.
 * @param epsilon - The maximum allowed difference for float comparison. Defaults to Number.EPSILON.
 * @returns True if values are deeply equal, false otherwise.
 */
export function deepEqual(val1: any, val2: any, epsilon: number = Number.EPSILON): boolean {
    if (val1 === val2) {
        return true;
    }

    if (Number.isNaN(val1) && Number.isNaN(val2)) {
        return true;
    }

    if (typeof val1 === 'number' && typeof val2 === 'number') {
        return Math.abs(val1 - val2) <= epsilon;
    }

    if (typeof val1 !== 'object' || val1 === null || typeof val2 !== 'object' || val2 === null) {
        return false;
    }

    if (Array.isArray(val1) && Array.isArray(val2)) {
        return deepEuqalArrays(val1, val2, epsilon);
    }

    if (typeof val1 === 'object' && typeof val2 === 'object') {
        return deepEqualObjects(val1, val2, epsilon);
    }

    return false;
}

/**
 * Deeply compares two arrays, including nested objects and arrays.
 * @param arr1 - The first array to compare.
 * @param arr2 - The second array to compare.
 * @param epsilon - The epsilon value for float comparison.
 * @returns True if arrays are deeply equal, false otherwise.
 */
function deepEuqalArrays(arr1: any[], arr2: any[], epsilon: number): boolean {
    if (arr1.length !== arr2.length) {
        return false;
    }
    for (let i = 0; i < arr1.length; i++) {
        if (!deepEqual(arr1[i], arr2[i], epsilon)) {
            return false;
        }
    }
    return true;
}

/**
 * Deeply compares two objects, including nested objects and arrays.
 * @param obj1 - The first object to compare.
 * @param obj2 - The second object to compare.
 * @param epsilon - The epsilon value for float comparison.
 * @returns True if objects are deeply equal, false otherwise.
 */
function deepEqualObjects(obj1: Record<string, any>, obj2: Record<string, any>, epsilon: number): boolean {
    const keys1 = Object.keys(obj1);
    const keys2 = Object.keys(obj2);

    if (keys1.length !== keys2.length) {
        return false;
    }

    for (const key of keys1) {
        if (!Object.prototype.hasOwnProperty.call(obj2, key) || !deepEqual(obj1[key], obj2[key], epsilon)) {
            return false;
        }
    }

    return true;
}
