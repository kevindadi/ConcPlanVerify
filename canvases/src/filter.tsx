import { createContext, useContext } from "react";

export const FilterContext = createContext("");

export function useRowFilter() {
  return useContext(FilterContext);
}
