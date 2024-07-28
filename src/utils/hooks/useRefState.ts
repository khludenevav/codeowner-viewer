import React, { useCallback, useRef, useState } from 'react';

/**
 * useState update is async, but we need actual value as soon as possible after change
 * that is where this hook useful.
 */
export default function useRefState<T>(
  initialValue: T | (() => T),
): [T, React.MutableRefObject<T>, React.Dispatch<React.SetStateAction<T>>] {
  const [value, setValue] = useState(initialValue);
  const ref = useRef(value);
  const setter: React.Dispatch<React.SetStateAction<T>> = useCallback(
    (newValueOrFunc: T | ((prevState: T) => T)) => {
      setValue(oldState => {
        let newState: T;
        if (typeof newValueOrFunc === 'function') {
          newState = (newValueOrFunc as unknown as (oldState: T) => T)(oldState);
        } else {
          newState = newValueOrFunc;
        }
        ref.current = newState;
        return newState;
      });
    },
    [],
  );
  return [value, ref, setter];
}
