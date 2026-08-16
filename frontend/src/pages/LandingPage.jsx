import Hero from '../components/marketing/Hero';
import AiSpotlight from '../components/marketing/spotlights/AiSpotlight';
import WorkflowTimeline from '../components/marketing/WorkflowTimeline';
import FeatureShowcase from '../components/marketing/FeatureShowcase';
import FinalCta from '../components/marketing/FinalCta';

export default function LandingPage() {
  return (
    <>
      <Hero />
      <FeatureShowcase />
      <AiSpotlight />
      <WorkflowTimeline />
      <FinalCta />
    </>
  );
}
