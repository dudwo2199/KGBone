// 작은 상태 저장소: 외부 라이브러리 없이 useSyncExternalStore로 구독한다.
import { useSyncExternalStore } from 'react';

export class Store<T extends object> {
  private listeners = new Set<() => void>();
  constructor(private value: T) {}

  get = (): T => this.value;

  set(patch: Partial<T> | ((s: T) => Partial<T>)): void {
    const p = typeof patch === 'function' ? patch(this.value) : patch;
    let changed = false;
    for (const k in p) {
      if (!Object.is((this.value as Record<string, unknown>)[k], (p as Record<string, unknown>)[k])) {
        changed = true;
        break;
      }
    }
    if (!changed) return;
    this.value = { ...this.value, ...p };
    for (const l of [...this.listeners]) l();
  }

  subscribe = (l: () => void): (() => void) => {
    this.listeners.add(l);
    return () => this.listeners.delete(l);
  };
}

/** 저장소의 일부를 구독한다. 선택자는 같은 상태에서 같은 값(참조)을 돌려줘야 한다. */
export function useStore<T extends object, S>(store: Store<T>, select: (s: T) => S): S {
  return useSyncExternalStore(store.subscribe, () => select(store.get()));
}
