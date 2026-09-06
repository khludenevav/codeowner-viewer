import originalDayjs, { type Dayjs as DayjsOriginal } from 'dayjs';
import advancedFormat from 'dayjs/plugin/advancedFormat';
import 'dayjs/plugin/advancedFormat';
import customParseFormat from 'dayjs/plugin/customParseFormat';
import 'dayjs/plugin/customParseFormat';
import duration from 'dayjs/plugin/duration';
import 'dayjs/plugin/duration';
import isSameOrAfter from 'dayjs/plugin/isSameOrAfter';
import 'dayjs/plugin/isSameOrAfter';
import isSameOrBefore from 'dayjs/plugin/isSameOrBefore';
import 'dayjs/plugin/isSameOrBefore';
import localizedFormat from 'dayjs/plugin/localizedFormat';
import 'dayjs/plugin/localizedFormat';
import minMax from 'dayjs/plugin/minMax';
import 'dayjs/plugin/minMax';
// do not uncomment. We redefine all it's types below
import objectSupport from 'dayjs/plugin/objectSupport';
import 'dayjs/plugin/objectSupport';
import relativeTime from 'dayjs/plugin/relativeTime';
import 'dayjs/plugin/relativeTime';
import utc from 'dayjs/plugin/utc';
import 'dayjs/plugin/utc';

originalDayjs.extend(utc);
originalDayjs.extend(objectSupport);
originalDayjs.extend(isSameOrBefore);
originalDayjs.extend(isSameOrAfter);
originalDayjs.extend(duration);
originalDayjs.extend(customParseFormat);
originalDayjs.extend(relativeTime);
originalDayjs.extend(localizedFormat);
originalDayjs.extend(minMax);
originalDayjs.extend(advancedFormat);

declare module 'dayjs' {
  // Basic max and min declaration returns Dayjs or null. This override removes null from return type
  function max(
    dayjs:
      | [DayjsOriginal]
      | [DayjsOriginal, DayjsOriginal]
      | [DayjsOriginal, DayjsOriginal, DayjsOriginal]
      | [DayjsOriginal, DayjsOriginal, DayjsOriginal, DayjsOriginal],
  ): DayjsOriginal;
  function min(
    dayjs:
      | [DayjsOriginal]
      | [DayjsOriginal, DayjsOriginal]
      | [DayjsOriginal, DayjsOriginal, DayjsOriginal]
      | [DayjsOriginal, DayjsOriginal, DayjsOriginal, DayjsOriginal],
  ): DayjsOriginal;
}

export { originalDayjs as dayjs };
export type Duration = ReturnType<typeof originalDayjs.duration>;
