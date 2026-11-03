import { invoke } from '@tauri-apps/api/core';
import { toast } from 'react-toastify';
import { useLibraryStore } from '../../store/useLibraryStore';
import { ImageFile, Invokes } from '../ui/AppProperties';
import { getImmichStackMembers } from './immichApi';

// Immich stacks are listed as one library entry (the cover, or the RAW of a
// RAW+JPG pair). Expanding one inserts its other files into the image list
// right after that entry, so they can be selected, opened and browsed in the
// filmstrip like any other image; collapsing removes them again.

const loading = new Set<string>();

/** Library entry that stands for the stack of an image (itself or its parent). */
export const stackEntryOf = (image: Pick<ImageFile, 'path' | 'immich_stack'>): string | null =>
  image.immich_stack ? (image.immich_stack.parent ?? image.path) : null;

const insertAfter = (list: ImageFile[], path: string, members: ImageFile[]): ImageFile[] => {
  const index = list.findIndex((image) => image.path === path);
  if (index < 0) return list;
  const present = new Set(list.map((image) => image.path));
  const fresh = members.filter((member) => !present.has(member.path));
  return [...list.slice(0, index + 1), ...fresh, ...list.slice(index + 1)];
};

const loadMemberExif = (members: ImageFile[]) => {
  const paths = members.map((member) => member.path);
  if (paths.length === 0) return;
  invoke<Record<string, Record<string, string>>>(Invokes.ReadExifForPaths, { paths })
    .then((exif) =>
      useLibraryStore.getState().setLibrary((state) => ({ exifByPath: { ...state.exifByPath, ...exif } })),
    )
    .catch((err) => console.error('Failed to read EXIF of stack members:', err));
};

const withRatings = (ratings: Record<string, number>, members: ImageFile[]) => {
  const next = { ...ratings };
  for (const member of members) {
    if (next[member.path] === undefined && member.rating !== undefined) next[member.path] = member.rating;
  }
  return next;
};

export function collapseImmichStack(path: string) {
  useLibraryStore.getState().setLibrary((state) => {
    const members = new Set(state.expandedImmichStacks[path] ?? []);
    const { [path]: _collapsed, ...expanded } = state.expandedImmichStacks;
    return {
      expandedImmichStacks: expanded,
      imageList: state.imageList.filter((image) => !members.has(image.path)),
      multiSelectedPaths: state.multiSelectedPaths.filter((p) => !members.has(p)),
      libraryActivePath:
        state.libraryActivePath && members.has(state.libraryActivePath) ? path : state.libraryActivePath,
    };
  });
}

export async function expandImmichStack(path: string) {
  if (loading.has(path) || useLibraryStore.getState().expandedImmichStacks[path]) return;
  loading.add(path);
  try {
    const members = await getImmichStackMembers(path);
    useLibraryStore.getState().setLibrary((state) => {
      if (!state.imageList.some((image) => image.path === path)) return {};
      return {
        imageList: insertAfter(state.imageList, path, members),
        imageRatings: withRatings(state.imageRatings, members),
        expandedImmichStacks: { ...state.expandedImmichStacks, [path]: members.map((member) => member.path) },
      };
    });
    loadMemberExif(members);
  } catch (err) {
    console.error('Failed to expand Immich stack:', err);
    toast.error(String(err));
  } finally {
    loading.delete(path);
  }
}

export function toggleImmichStack(path: string) {
  if (useLibraryStore.getState().expandedImmichStacks[path]) collapseImmichStack(path);
  else return expandImmichStack(path);
}

/**
 * For a freshly loaded Immich listing: keeps the stacks the user had expanded
 * (when the same album is refreshed) or forgets them (another album).
 */
export async function withExpandedImmichStacks(files: ImageFile[], keepExpanded: boolean): Promise<ImageFile[]> {
  const { expandedImmichStacks, setLibrary } = useLibraryStore.getState();
  const wanted = keepExpanded ? Object.keys(expandedImmichStacks) : [];
  const present = new Set(files.map((file) => file.path));
  const expanded: Record<string, string[]> = {};
  let list = files;
  const allMembers: ImageFile[] = [];
  for (const path of wanted) {
    if (!present.has(path)) continue;
    try {
      const members = await getImmichStackMembers(path);
      list = insertAfter(list, path, members);
      expanded[path] = members.map((member) => member.path);
      allMembers.push(...members);
    } catch (err) {
      console.error('Failed to keep Immich stack expanded:', err);
    }
  }
  setLibrary({ expandedImmichStacks: expanded });
  loadMemberExif(allMembers);
  return list;
}

/**
 * Puts the files of each expanded stack right after its entry, whatever the
 * sort order. Files whose entry is filtered out stay where they are.
 */
export function placeImmichStackMembers(list: ImageFile[], expanded: Record<string, string[]>): ImageFile[] {
  const entries = Object.keys(expanded);
  if (entries.length === 0) return list;
  const listed = new Set(list.map((image) => image.path));
  const byPath = new Map(list.map((image) => [image.path, image]));
  const placed = new Set<string>();
  for (const entry of entries) {
    if (!listed.has(entry)) continue;
    for (const member of expanded[entry]) placed.add(member);
  }
  if (placed.size === 0) return list;
  const result: ImageFile[] = [];
  for (const image of list) {
    if (placed.has(image.path)) continue;
    result.push(image);
    const members = expanded[image.path];
    if (!members) continue;
    for (const member of members) {
      const file = byPath.get(member);
      if (file) result.push(file);
    }
  }
  return result;
}
