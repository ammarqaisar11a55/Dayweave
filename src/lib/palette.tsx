import {
  Apple,
  Bed,
  Bike,
  Book,
  BookOpen,
  Brain,
  Briefcase,
  Code,
  Coffee,
  Droplets,
  Dumbbell,
  Flame,
  Flower2,
  Folder,
  Footprints,
  Gamepad2,
  GraduationCap,
  Heart,
  Home,
  Languages,
  Leaf,
  type LucideIcon,
  Moon,
  Music,
  Palette,
  Pencil,
  PiggyBank,
  Rocket,
  Salad,
  Sparkles,
  Star,
  Sun,
  Target,
  Wind,
} from "lucide-react";
import type { CSSProperties } from "react";

export const COLORS = ["orange", "red", "amber", "green", "teal", "blue", "violet", "pink", "slate"] as const;

export const colorVar = (name: string | null | undefined) => `var(--c-${name ?? "slate"})`;

export const softStyle = (name: string | null | undefined): CSSProperties => ({
  color: colorVar(name),
  backgroundColor: `color-mix(in srgb, ${colorVar(name)} 14%, transparent)`,
});

export const ICONS: Record<string, LucideIcon> = {
  sparkles: Sparkles,
  target: Target,
  book: Book,
  "book-open": BookOpen,
  brain: Brain,
  code: Code,
  "graduation-cap": GraduationCap,
  languages: Languages,
  pencil: Pencil,
  dumbbell: Dumbbell,
  footprints: Footprints,
  bike: Bike,
  heart: Heart,
  droplets: Droplets,
  apple: Apple,
  salad: Salad,
  coffee: Coffee,
  bed: Bed,
  moon: Moon,
  sun: Sun,
  wind: Wind,
  leaf: Leaf,
  flower: Flower2,
  flame: Flame,
  music: Music,
  palette: Palette,
  gamepad: Gamepad2,
  home: Home,
  briefcase: Briefcase,
  folder: Folder,
  rocket: Rocket,
  star: Star,
  "piggy-bank": PiggyBank,
};

export function iconFor(name: string | null | undefined): LucideIcon {
  return ICONS[name ?? ""] ?? Sparkles;
}
