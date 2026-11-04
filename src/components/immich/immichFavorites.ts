import { toast } from 'react-toastify';
import { useLibraryStore } from '../../store/useLibraryStore';
import { ImageFile } from '../ui/AppProperties';
import { setImmichFavorite } from './immichApi';
import { stackEntryOf } from './immichStacks';
import { useImmichStore } from './useImmichStore';

// Immich favorites (asset.isFavorite). An entry that stands for a stack is a favorite when any of its files
// is. Marking it marks the file Immich shows for it; unmarking it unmarks every favorite file of the stack.

const basePath = (path: string) => path.split('?vc=')[0];

export const isImmichImage = (image: Pick<ImageFile, 'immich_favorite'> | undefined) =>
  image?.immich_favorite !== undefined && image?.immich_favorite !== null;

const pending = new Set<string>();

/**
 * Marks the given library entries as Immich favorites (or unmarks them). Without `favorite`, toggles: all
 * become favorites unless they all are already.
 */
export async function setImmichFavorites(paths: string[], favorite?: boolean) {
  const { imageList, expandedImmichStacks } = useLibraryStore.getState();
  const byPath = new Map(imageList.map((image) => [image.path, image]));
  const images = paths.map((path) => byPath.get(path)).filter((image): image is ImageFile => isImmichImage(image));
  if (images.length === 0) return;
  const key = images.map((image) => image.path).join('\n');
  if (pending.has(key)) return;
  const target = favorite ?? !images.every((image) => image.immich_favorite);

  // The other files of the stacks involved can change too (entry <-> expanded files).
  const refresh = new Set<string>();
  for (const image of images) {
    const entry = stackEntryOf(image);
    if (!entry) continue;
    refresh.add(entry);
    for (const member of expandedImmichStacks[entry] ?? []) refresh.add(member);
  }

  pending.add(key);
  try {
    const states = await setImmichFavorite(
      images.map((image) => image.path),
      target,
      [...refresh],
    );
    const byBase = new Map(Object.entries(states).map(([path, state]) => [basePath(path), state]));
    useLibraryStore.getState().setLibrary((state) => ({
      imageList: state.imageList.map((image) => {
        const next = byBase.get(basePath(image.path));
        return next === undefined || next === image.immich_favorite ? image : { ...image, immich_favorite: next };
      }),
    }));
    useImmichStore.getState().loadFavoritesCount();
  } catch (err) {
    console.error('Failed to update Immich favorites:', err);
    toast.error(String(err));
  } finally {
    pending.delete(key);
  }
}
